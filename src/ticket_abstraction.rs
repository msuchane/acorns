use std::convert::From;
use std::fmt;

use color_eyre::eyre::{eyre, Context, Result};

use bugzilla_query::Bug;
use jira_query::Issue;

use crate::config::{tracker, TicketQuery};
use crate::extra_fields::ExtraFields;

/// An abstract ticket representation that generalizes over Bugzilla, Jira, and any other issue trackers.
#[derive(Clone, Debug)]
pub struct AbstractTicket {
    pub id: TicketId,
    pub summary: String,
    // TODO: Find out how to get the bug description from comment#0 with Bugzilla
    pub description: Option<String>,
    pub doc_type: Option<String>,
    pub doc_text: Option<String>,
    pub docs_contact: Option<String>,
    pub status: String,
    pub is_open: bool,
    pub priority: String,
    pub url: String,
    pub assignee: String,
    pub components: Vec<String>,
    pub product: String,
    pub labels: Option<Vec<String>>,
    pub flags: Option<Vec<String>>,
    pub target_release: Option<String>,
    pub subsystems: Vec<String>,
    pub groups: Option<Vec<String>>,
    pub public: bool,
    pub requires_doc_text: DocTextStatus,
    pub duplicates: Vec<AbstractTicket>,
}

/// An identification of the original ticket on the issue tracker.
#[derive(Clone, Debug)]
pub struct TicketId {
    pub key: String,
    pub tracker: tracker::Service,
}

/// The status or progress of the release note.
#[derive(Clone, Debug)]
pub enum DocTextStatus {
    Approved,
    InProgress,
    NoDocumentation,
}

impl From<&str> for DocTextStatus {
    fn from(string: &str) -> Self {
        match string {
            "+" => Self::Approved,
            "-" => Self::NoDocumentation,
            _ => Self::InProgress,
        }
    }
}

impl fmt::Display for DocTextStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let display = match self {
            Self::Approved => "RDT+",
            Self::InProgress => "RDT?",
            Self::NoDocumentation => "RDT-",
        };
        write!(f, "{}", display)
    }
}

impl From<Bug> for AbstractTicket {
    fn from(bug: Bug) -> Self {
        AbstractTicket {
            id: TicketId {
                key: bug.id.to_string(),
                tracker: tracker::Service::Bugzilla,
            },
            // TODO: Find out how to get the bug description from comment#0 with Bugzilla
            description: None,
            doc_type: bug.doc_type(),
            doc_text: bug.doc_text(),
            docs_contact: Some(bug.docs_contact),
            summary: bug.summary,
            status: bug.status,
            is_open: bug.is_open,
            priority: bug.priority,
            url: bug.url,
            assignee: bug.assigned_to,
            components: bug.component,
            product: bug.product,
            // Bugzilla has no labels
            labels: None,
            // TODO: Implement flags as strings
            flags: bug.flags.map(|flags| {
                flags
                    .into_iter()
                    .map(|flag| format!("{}: {}", flag.name, flag.status))
                    .collect()
            }),
            target_release: bug
                .extra
                .get("cf_internal_target_release")
                .map(|itr| itr.as_str().unwrap().to_string()),
            subsystems: bug
                .extra
                .get("pool")
                .and_then(|pool| pool.get("team"))
                .and_then(|team| team.get("name"))
                .map_or(Vec::new(), |name| vec![name.as_str().unwrap().to_string()]),
            // A bug is public if no groups are set for it.
            public: bug.groups.is_empty(),
            groups: Some(bug.groups),
            // TODO: Implement RDT
            requires_doc_text: DocTextStatus::InProgress,
            duplicates: Vec::new(),
        }
    }
}

impl From<Issue> for AbstractTicket {
    fn from(issue: Issue) -> Self {
        AbstractTicket {
            doc_type: issue.doc_type(),
            doc_text: issue.doc_text(),
            docs_contact: issue
                .fields
                .extra
                .get("customfield_12317336")
                .and_then(|cf| cf.get("emailAddress"))
                .map(|value| value.as_str().unwrap().to_string()),
            id: TicketId {
                key: issue.key,
                tracker: tracker::Service::Jira,
            },
            summary: issue.fields.summary,
            description: issue.fields.description,
            is_open: &issue.fields.status.name != "Closed",
            status: issue.fields.status.name,
            priority: issue.fields.priority.name,
            url: issue.self_link,
            assignee: issue.fields.assignee.name,
            components: issue
                .fields
                .components
                .into_iter()
                .map(|c| c.name)
                .collect(),
            product: issue.fields.project.name,
            labels: Some(issue.fields.labels),
            // Jira does not support flags
            flags: None,
            // TODO: Is the first fix version in the list the one that we want?
            target_release: issue
                .fields
                .fix_versions
                .into_iter()
                .next()
                .map(|version| version.name),
            // TODO: Handle the errors more safely, without unwraps.
            subsystems: issue
                .fields
                .extra
                // This is the "Pool Team" field.
                .get("customfield_12317259")
                .and_then(|ssts| ssts.as_array())
                .unwrap()
                .iter()
                .map(|sst| sst.get("value").unwrap().as_str().unwrap().to_string())
                .collect(),
            // Jira does not recognize groups in the Bugzilla way. This might change.
            groups: None,
            // TODO: Implement public
            public: false,
            // TODO: This field should be configurable
            requires_doc_text: issue
                .fields
                .extra
                .get("customfield_12317337")
                .and_then(|rdt| rdt.as_str())
                .map_or(DocTextStatus::InProgress, DocTextStatus::from),
            duplicates: Vec::new(),
        }
    }
}

/// Process the configured ticket queries into abstract tickets,
/// sorted in the original order as found in the config file.
pub fn from_queries(
    queries: &[TicketQuery],
    trackers: &tracker::Config,
) -> Result<Vec<AbstractTicket>> {
    let tickets = unsorted_tickets(queries, trackers)?;

    // Sort tickets to the order in the config file:
    let mut sorted_tickets: Vec<AbstractTicket> = Vec::new();

    for query in queries {
        let mut matching_tickets: Vec<AbstractTicket> = tickets
            .iter()
            .filter(|t| query.tracker == t.id.tracker && query.key == t.id.key)
            // TODO: Try to avoid the cloning.
            .cloned()
            .collect();
        // A query might result in no tickets. For example, Bugzilla silently ignores nonexistent IDs.
        // In that case, report the error and immediately exit the program.
        if matching_tickets.is_empty() {
            return Err(eyre!("Query produced no tickets: {:#?}", query));
        }
        sorted_tickets.append(&mut matching_tickets);
    }

    Ok(sorted_tickets)
}

// TODO: Move these two functions to a more appropriate place, possibly a new module.
/// Prepare a client to access Bugzilla.
fn bz_instance(trackers: &tracker::Config) -> Result<bugzilla_query::BzInstance> {
    let api_key = if let Some(key) = &trackers.bugzilla.api_key {
        key.clone()
    } else {
        // TODO: Store the name of the variable in a constant, or make it configurable.
        std::env::var("BZ_API_KEY").context("Set the BZ_API_KEY environment variable.")?
    };

    Ok(bugzilla_query::BzInstance {
        host: trackers.bugzilla.host.clone(),
        auth: bugzilla_query::Auth::ApiKey(api_key),
    })
}
/// Prepare a client to access Jira.
fn jira_instance(trackers: &tracker::Config) -> Result<jira_query::JiraInstance> {
    let api_key = if let Some(key) = &trackers.jira.api_key {
        key.clone()
    } else {
        // TODO: Store the name of the variable in a constant, or make it configurable.
        std::env::var("JIRA_API_KEY").context("Set the JIRA_API_KEY environment variable.")?
    };

    Ok(jira_query::JiraInstance {
        host: trackers.jira.host.clone(),
        auth: jira_query::Auth::ApiKey(api_key),
    })
}

/// Process the configured ticket queries into abstract tickets,
/// sorted in no particular order, which depends on the response from the issue tracker.
fn unsorted_tickets(
    queries: &[TicketQuery],
    trackers: &tracker::Config,
) -> Result<Vec<AbstractTicket>> {
    let bugzilla_queries = queries
        .iter()
        .filter(|t| t.tracker == tracker::Service::Bugzilla);
    let jira_queries = queries
        .iter()
        .filter(|t| t.tracker == tracker::Service::Jira);

    let bz_instance = bz_instance(trackers)?;

    let bugs = bz_instance
        .bugs(
            &bugzilla_queries
                .map(|q| q.key.as_str())
                .collect::<Vec<&str>>(),
        )
        .context("Failed to download tickets from Bugzilla.")?;

    let jira_instance = jira_instance(trackers)?;

    let issues = jira_instance
        .issues(&jira_queries.map(|q| q.key.as_str()).collect::<Vec<&str>>())
        .context("Failed to download tickets from Jira.")?;

    let tickets_from_bugzilla = bugs.into_iter().map(AbstractTicket::from);
    let tickets_from_jira = issues.into_iter().map(AbstractTicket::from);

    Ok(tickets_from_bugzilla.chain(tickets_from_jira).collect())
}

/// Process a single ticket specified using the `ticket` subcommand.
pub fn from_args(
    service: tracker::Service,
    id: &str,
    host: &str,
    api_key: &str,
) -> Result<AbstractTicket> {
    match service {
        tracker::Service::Jira => {
            let jira_instance = jira_query::JiraInstance {
                host: host.to_string(),
                auth: jira_query::Auth::ApiKey(api_key.to_string()),
            };

            let issue = jira_instance.issue(id)?;
            Ok(issue.into())
        }
        tracker::Service::Bugzilla => {
            let bz_instance = bugzilla_query::BzInstance {
                host: host.to_string(),
                auth: bugzilla_query::Auth::ApiKey(api_key.to_string()),
            };

            let bug = bz_instance.bug(id)?;
            Ok(bug.into())
        }
    }
}
