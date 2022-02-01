use std::convert::From;

use color_eyre::eyre::{Context, Result};
use log::error;

use bugzilla_query::Bug;
use jira_query::JiraIssue;

use crate::config::{tracker, TicketQuery};

#[derive(Clone, Debug)]
pub struct AbstractTicket {
    pub id: TicketId,
    pub summary: String,
    // TODO: Find out how to get the bug description from comment#0 with Bugzilla
    pub description: Option<String>,
    pub doc_type: String,
    pub doc_text: Option<String>,
    pub docs_contact: String,
    pub release_note: Option<String>,
    pub status: String,
    pub is_open: bool,
    pub priority: String,
    pub url: String,
    pub assignee: String,
    pub components: Vec<String>,
    pub product: String,
    pub labels: Option<Vec<String>>,
    pub flags: Option<Vec<String>>,
    pub target_release: String,
    pub subsystems: Vec<String>,
    pub groups: Option<Vec<String>>,
    pub public: bool,
    pub requires_doc_text: DocTextStatus,
    pub duplicates: Vec<AbstractTicket>,
}

#[derive(Clone, Debug)]
pub struct TicketId {
    pub key: String,
    pub tracker: tracker::Service,
}

#[derive(Clone, Debug)]
pub enum DocTextStatus {
    Approved,
    InProgress,
    NoDocumentation,
}

impl From<Bug> for AbstractTicket {
    fn from(bug: Bug) -> Self {
        AbstractTicket {
            id: TicketId {
                key: bug.id.to_string(),
                tracker: tracker::Service::Bugzilla,
            },
            summary: bug.summary,
            // TODO: Find out how to get the bug description from comment#0 with Bugzilla
            description: None,
            // TODO: These two fields should be configurable by tracker.
            // Also, handle the errors properly.
            doc_type: bug
                .extra
                .get("cf_doc_type")
                .unwrap()
                .as_str()
                .unwrap()
                .to_string(),
            doc_text: bug
                .extra
                .get("cf_release_notes")
                .map(|cf_release_notes| cf_release_notes.as_str().unwrap().to_string()),
            docs_contact: bug.docs_contact,
            release_note: None,
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
            flags: None,
            target_release: bug
                .extra
                .get("cf_internal_target_release")
                .unwrap()
                .as_str()
                .unwrap()
                .to_string(),
            // TODO: Implement SST. The path is extra.pool.team.name
            subsystems: vec!["SST".to_string()],
            groups: Some(bug.groups),
            // TODO: Implement public
            public: false,
            // TODO: Implement RDT
            requires_doc_text: DocTextStatus::InProgress,
            duplicates: Vec::new(),
        }
    }
}

impl From<JiraIssue> for AbstractTicket {
    fn from(issue: JiraIssue) -> Self {
        AbstractTicket {
            id: TicketId {
                key: issue.key,
                tracker: tracker::Service::Jira,
            },
            summary: issue.fields.summary,
            description: issue.fields.description,
            // TODO: These two fields should be configurable by tracker.
            // Also, handle the errors properly.
            doc_type: issue
                .fields
                .extra
                .get("customfield_12317310")
                .unwrap()
                .get("value")
                .unwrap()
                .as_str()
                .unwrap()
                .to_string(),
            // This chain of `and_then` and `map` handles the two consecutive Options:
            // The result is a String only when neither Option is None.
            // The first method is `and_then` rather than `map` to avoid a nested Option.
            doc_text: issue.fields.extra.get("customfield_12317322").and_then(
                |customfield_12317322| {
                    customfield_12317322
                        .get("value")
                        .map(|value| value.as_str().unwrap().to_string())
                },
            ),
            docs_contact: issue
                .fields
                .extra
                .get("customfield_12317336")
                .unwrap()
                .get("emailAddress")
                .unwrap()
                .as_str()
                .unwrap()
                .to_string(),
            release_note: None,
            is_open: &issue.fields.status.name != "Closed",
            status: issue.fields.status.name,
            priority: issue.fields.priority.name,
            url: issue.self_link,
            assignee: issue.fields.assignee.email_address,
            // TODO: Implement components.
            // The call would be issue.fields.components.map(|&c| c.name),
            // but I'm getting an iterator trait error.
            components: Vec::new(),
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
                .expect("No fix version set for an issue.")
                .name,
            // TODO: Implement SSTs. Previously, we used labels, but now the menu is available.
            subsystems: vec!["SST".to_string()],
            // Jira does not recognize groups in the Bugzilla way. This might change.
            groups: None,
            // TODO: Implement public
            public: false,
            // TODO: Implement RDT
            requires_doc_text: DocTextStatus::InProgress,
            duplicates: Vec::new(),
        }
    }
}

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
        if matching_tickets.is_empty() {
            error!("Query produced no tickets: {:#?}", query);
        }
        sorted_tickets.append(&mut matching_tickets);
    }

    Ok(sorted_tickets)
}

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

    let bugs = bugzilla_query::bugs(
        &trackers.bugzilla.host,
        &bugzilla_queries
            .map(|q| q.key.as_str())
            .collect::<Vec<&str>>(),
        &trackers.bugzilla.api_key,
    )?;
    let issues = jira_query::issues(
        &trackers.jira.host,
        &jira_queries.map(|q| q.key.as_str()).collect::<Vec<&str>>(),
        &trackers.jira.api_key,
    )?;

    let tickets_from_bugzilla = bugs.into_iter().map(|b| AbstractTicket::from(b));
    let tickets_from_jira = issues.into_iter().map(|i| AbstractTicket::from(i));

    Ok(tickets_from_bugzilla.chain(tickets_from_jira).collect())
}