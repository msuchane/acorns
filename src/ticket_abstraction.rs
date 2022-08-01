use std::fmt;
use std::rc::Rc;
use std::string::ToString;
use std::sync::Arc;

use bugzilla_query::{Bug, Component};
use color_eyre::eyre::{bail, Result};
use jira_query::Issue;

use crate::config::{tracker, TicketQuery};
use crate::extra_fields::{DocTextStatus, ExtraFields};
use crate::tracker_access::{self, AnnotatedTicket};

/// An abstract ticket representation that generalizes over Bugzilla, Jira, and any other issue trackers.
#[derive(Clone, Debug)]
pub struct AbstractTicket {
    pub id: Rc<TicketId>,
    pub summary: String,
    // TODO: Find out how to get the bug description from comment#0 with Bugzilla
    pub description: Option<String>,
    pub doc_type: String,
    pub doc_text: String,
    pub docs_contact: String,
    pub status: String,
    pub is_open: bool,
    pub priority: String,
    pub url: String,
    pub assignee: Option<String>,
    pub components: Vec<String>,
    pub product: String,
    pub labels: Option<Vec<String>>,
    pub flags: Option<Vec<String>>,
    pub target_releases: Vec<String>,
    pub subsystems: Vec<String>,
    pub groups: Option<Vec<String>>,
    pub public: bool,
    pub doc_text_status: DocTextStatus,
    pub duplicates: Vec<AbstractTicket>,
}

/// An identification of the original ticket on the issue tracker.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct TicketId {
    pub key: String,
    pub tracker: tracker::Service,
}

impl fmt::Display for TicketId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}:{}", &self.tracker, &self.key)
    }
}

pub trait IntoAbstract {
    /// Converts a Bugzilla bug or a Jira ticket to `AbstractTicket`.
    /// Consumes the original ticket.
    fn into_abstract(self, config: &tracker::Instance) -> Result<AbstractTicket>;
}

impl IntoAbstract for Bug {
    fn into_abstract(self, tracker: &tracker::Instance) -> Result<AbstractTicket> {
        let ticket = AbstractTicket {
            id: Rc::new(TicketId {
                key: self.id.to_string(),
                tracker: tracker::Service::Bugzilla,
            }),
            // TODO: Find out how to get the bug description from comment#0 with Bugzilla
            description: None,
            doc_type: self.doc_type(&tracker.fields)?,
            doc_text: self.doc_text(&tracker.fields)?,
            target_releases: self.target_releases(&tracker.fields)?,
            subsystems: self.subsystems(&tracker.fields)?,
            doc_text_status: self.doc_text_status(&tracker.fields)?,
            docs_contact: self.docs_contact(&tracker.fields)?,
            url: self.url(tracker),
            summary: self.summary,
            status: self.status,
            is_open: self.is_open,
            priority: self.priority,
            // Bugs are always assigned to someone.
            assignee: Some(self.assigned_to),
            components: match self.component {
                Component::One(c) => vec![c],
                Component::Many(cs) => cs,
            },
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
        };

        Ok(ticket)
    }
}

impl IntoAbstract for Issue {
    fn into_abstract(self, tracker: &tracker::Instance) -> Result<AbstractTicket> {
        let ticket = AbstractTicket {
            doc_type: self.doc_type(&tracker.fields)?,
            doc_text: self.doc_text(&tracker.fields)?,
            // The target release is non-essential. Discard the error and store as Option.
            target_releases: self.target_releases(&tracker.fields)?,
            doc_text_status: self.doc_text_status(&tracker.fields)?,
            docs_contact: self.docs_contact(&tracker.fields)?,
            subsystems: self.subsystems(&tracker.fields)?,
            url: self.url(tracker),
            // The ID in particular is wrapped in Rc because it's involved in various filters
            // and comparisons where ownership is complicated.
            id: Rc::new(TicketId {
                key: self.key,
                tracker: tracker::Service::Jira,
            }),
            summary: self.fields.summary,
            description: self.fields.description,
            is_open: &self.fields.status.name != "Closed",
            status: self.fields.status.name,
            priority: self.fields.priority.name,
            // Issues might not be assigned to anyone.
            assignee: self.fields.assignee.map(|a| a.name),
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
        };

        Ok(ticket)
    }
}

/// Process the configured ticket queries into abstract tickets,
/// sorted in the original order as found in the config file.
pub fn from_queries(
    queries: &[Arc<TicketQuery>],
    trackers: &tracker::Config,
) -> Result<Vec<AbstractTicket>> {
    let annotated_tickets = tracker_access::unsorted_tickets(queries, trackers)?;

    // Sort the tickets according to the order in the config file.
    let sorted_tickets = sort_tickets(queries, &annotated_tickets)?;

    // Strip the query from the ticket. The query has served its full purpose.
    Ok(sorted_tickets.into_iter().map(|at| at.ticket).collect())
}

/// Sort tickets to the order specified in the tickets configuration file.
pub fn sort_tickets(
    queries: &[Arc<TicketQuery>],
    tickets: &[AnnotatedTicket],
) -> Result<Vec<AnnotatedTicket>> {
    let mut sorted_tickets: Vec<AnnotatedTicket> = Vec::new();

    // Go query by query. Queries are still sorted the same as in the config file. Use their order.
    for query in queries {
        // Find the indices of all tickets that match this query.
        // We're dealing with indices because that enables us to move a ticket from one Vec to another
        // using the Vec::swap_remove method, which takes an index as its argument.
        let mut matching_tickets: Vec<AnnotatedTicket> = tickets
            .iter()
            .filter(|at| query == &at.query)
            // TODO: Revisit whether this clone is necessary.
            // Note to self: Do not use Vec::swap_remove, it changes the Vec ordering and size at runtime.
            .cloned()
            .collect();

        // A query might result in no tickets. For example, Bugzilla silently ignores nonexistent IDs.
        // In that case, report the error and immediately exit the program.
        if matching_tickets.is_empty() {
            bail!("Query produced no tickets: {:#?}", query);
        }

        // Insert tickets that match this query into the sorted Vec.
        sorted_tickets.append(&mut matching_tickets);
    }

    Ok(sorted_tickets)
}
