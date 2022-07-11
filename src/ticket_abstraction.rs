use std::convert::From;
use std::string::ToString;

use bugzilla_query::Bug;
use color_eyre::eyre::{bail, Result};
use jira_query::Issue;

use crate::config::{tracker, TicketQuery};
use crate::extra_fields::{DocTextStatus, ExtraFields};
use crate::tracker_access;

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

pub trait IntoAbstract {
    fn into_abstract(self, config: &tracker::Fields) -> AbstractTicket;
}

impl IntoAbstract for Bug {
    fn into_abstract(self, config: &tracker::Fields) -> AbstractTicket {
        AbstractTicket {
            id: TicketId {
                key: self.id.to_string(),
                tracker: tracker::Service::Bugzilla,
            },
            // TODO: Find out how to get the bug description from comment#0 with Bugzilla
            description: None,
            doc_type: self.doc_type(config),
            doc_text: self.doc_text(config),
            target_release: self.target_release(config),
            subsystems: self.subsystems(config),
            doc_text_status: self.doc_text_status(config),
            docs_contact: Some(self.docs_contact),
            summary: self.summary,
            status: self.status,
            is_open: self.is_open,
            priority: self.priority,
            url: self.url,
            assignee: self.assigned_to,
            components: self.component,
            product: self.product,
            // Bugzilla has no labels
            labels: None,
            // Convert all flags to `name: value` strings.
            flags: self
                .flags
                .map(|flags| flags.into_iter().map(|flag| flag.to_string()).collect()),
            // A bug is public if no groups are set for it.
            public: self.groups.is_empty(),
            groups: Some(self.groups),
            duplicates: Vec::new(),
        }
    }
}

impl IntoAbstract for Issue {
    fn into_abstract(self, config: &tracker::Fields) -> AbstractTicket {
        AbstractTicket {
            doc_type: self.doc_type(&config),
            doc_text: self.doc_text(&config),
            target_release: self.target_release(&config),
            doc_text_status: self.doc_text_status(&config),
            docs_contact: self
                .fields
                .extra
                .get("customfield_12317336")
                .and_then(|cf| cf.get("emailAddress"))
                .map(|value| value.as_str().unwrap().to_string()),
            subsystems: self.subsystems(&config),
            id: TicketId {
                key: self.key,
                tracker: tracker::Service::Jira,
            },
            summary: self.fields.summary,
            description: self.fields.description,
            is_open: &self.fields.status.name != "Closed",
            status: self.fields.status.name,
            priority: self.fields.priority.name,
            url: self.self_link,
            assignee: self.fields.assignee.name,
            components: self.fields.components.into_iter().map(|c| c.name).collect(),
            product: self.fields.project.name,
            labels: Some(self.fields.labels),
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
    let tickets = tracker_access::unsorted_tickets(queries, trackers)?;

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
