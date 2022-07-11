use std::convert::From;
use std::string::ToString;

use color_eyre::eyre::{bail, Context, Result};

use bugzilla_query::Bug;
use jira_query::Issue;

use crate::config::{tracker, TicketQuery};
use crate::extra_fields::{DocTextStatus, ExtraFields};

// The number of items in a single Jira query.
// All Jira queries are processed in chunks of this size.
// This prevents hitting the maximum allowed request size set in the Jira instance.
// TODO: Make this configurable.
const JIRA_CHUNK_SIZE: u32 = 30;

// Always include these fields in Bugzilla requests. We process some of their content.
const BZ_INCLUDED_FIELDS: &[&str; 3] = &["_default", "pool", "flags"];

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
    pub doc_text_status: DocTextStatus,
    pub duplicates: Vec<AbstractTicket>,
}

/// An identification of the original ticket on the issue tracker.
#[derive(Clone, Debug)]
pub struct TicketId {
    pub key: String,
    pub tracker: tracker::Service,
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
            target_release: bug.target_release(),
            subsystems: bug.subsystems(),
            doc_text_status: bug.doc_text_status(),
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
            // Convert all flags to `name: value` strings.
            flags: bug
                .flags
                .map(|flags| flags.into_iter().map(|flag| flag.to_string()).collect()),
            // A bug is public if no groups are set for it.
            public: bug.groups.is_empty(),
            groups: Some(bug.groups),
            duplicates: Vec::new(),
        }
    }
}

impl From<Issue> for AbstractTicket {
    fn from(issue: Issue) -> Self {
        AbstractTicket {
            doc_type: issue.doc_type(),
            doc_text: issue.doc_text(),
            target_release: issue.target_release(),
            doc_text_status: issue.doc_text_status(),
            docs_contact: issue
                .fields
                .extra
                .get("customfield_12317336")
                .and_then(|cf| cf.get("emailAddress"))
                .map(|value| value.as_str().unwrap().to_string()),
            subsystems: issue.subsystems(),
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
            // Jira does not recognize groups in the Bugzilla way. This might change.
            groups: None,
            // TODO: Implement public
            public: false,
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
            bail!("Query produced no tickets: {:#?}", query);
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

    Ok(
        bugzilla_query::BzInstance::at(trackers.bugzilla.host.clone())?
            .authenticate(bugzilla_query::Auth::ApiKey(api_key))?
            .paginate(bugzilla_query::Pagination::Unlimited)
            .include_fields(BZ_INCLUDED_FIELDS.iter().map(ToString::to_string).collect()),
    )
}
/// Prepare a client to access Jira.
fn jira_instance(trackers: &tracker::Config) -> Result<jira_query::JiraInstance> {
    let api_key = if let Some(key) = &trackers.jira.api_key {
        key.clone()
    } else {
        // TODO: Store the name of the variable in a constant, or make it configurable.
        std::env::var("JIRA_API_KEY").context("Set the JIRA_API_KEY environment variable.")?
    };

    Ok(jira_query::JiraInstance::at(trackers.jira.host.clone())?
        .authenticate(jira_query::Auth::ApiKey(api_key))?
        .paginate(jira_query::Pagination::ChunkSize(JIRA_CHUNK_SIZE)))
}

// TODO: Consider adding progress bars here. Investigate these libraries:
// * https://crates.io/crates/progressing
// * https://crates.io/crates/linya
// * https://crates.io/crates/indicatif
/// Process the configured ticket queries into abstract tickets,
/// sorted in no particular order, which depends on the response from the issue tracker.
///
/// Downloads from Bugzilla and from Jira in parallel.
#[tokio::main]
async fn unsorted_tickets(
    queries: &[TicketQuery],
    trackers: &tracker::Config,
) -> Result<Vec<AbstractTicket>> {
    // Download from Bugzilla and from Jira in parallel:
    let bugs = bugs(queries, trackers);
    let issues = issues(queries, trackers);

    // Wait until both downloads have finished:
    let (bugs, issues) = tokio::join!(bugs, issues);

    // Convert bugs and issues into abstract tickets:
    let tickets_from_bugzilla = bugs?.into_iter().map(AbstractTicket::from);
    let tickets_from_jira = issues?.into_iter().map(AbstractTicket::from);

    Ok(tickets_from_bugzilla.chain(tickets_from_jira).collect())
}

/// Download all configured bugs from Bugzilla.
async fn bugs(queries: &[TicketQuery], trackers: &tracker::Config) -> Result<Vec<Bug>> {
    let bugzilla_queries = queries
        .iter()
        .filter(|&t| t.tracker == tracker::Service::Bugzilla);

    let bz_instance = bz_instance(trackers)?;

    log::info!("Downloading bugs from Bugzilla.");

    let bugs = bz_instance
        .bugs(
            &bugzilla_queries
                .map(|q| q.key.as_str())
                .collect::<Vec<&str>>(),
        )
        // This enables the download concurrency:
        .await
        .context("Failed to download tickets from Bugzilla.")?;

    log::info!("Finished downloading from Bugzilla.");

    Ok(bugs)
}

/// Download all configured issues from Jira.
async fn issues(queries: &[TicketQuery], trackers: &tracker::Config) -> Result<Vec<Issue>> {
    let jira_queries = queries
        .iter()
        .filter(|&t| t.tracker == tracker::Service::Jira);

    let jira_instance = jira_instance(trackers)?;

    log::info!("Downloading issues from Jira.");

    let issues = jira_instance
        .issues(&jira_queries.map(|q| q.key.as_str()).collect::<Vec<&str>>())
        // This enables the download concurrency:
        .await
        .context("Failed to download tickets from Jira.")?;

    log::info!("Finished downloading from Jira.");

    Ok(issues)
}

/// Process a single ticket specified using the `ticket` subcommand.
#[tokio::main]
pub async fn from_args(
    service: tracker::Service,
    id: &str,
    host: &str,
    api_key: &str,
) -> Result<AbstractTicket> {
    match service {
        tracker::Service::Jira => {
            let jira_instance = jira_query::JiraInstance::at(host.to_string())?
                .authenticate(jira_query::Auth::ApiKey(api_key.to_string()))?;

            let issue = jira_instance.issue(id).await?;
            Ok(issue.into())
        }
        tracker::Service::Bugzilla => {
            let bz_instance = bugzilla_query::BzInstance::at(host.to_string())?
                .authenticate(bugzilla_query::Auth::ApiKey(api_key.to_string()))?
                .include_fields(BZ_INCLUDED_FIELDS.iter().map(ToString::to_string).collect());

            let bug = bz_instance.bug(id).await?;
            Ok(bug.into())
        }
    }
}
