use bugzilla_query::Bug;
use jira_query::JiraIssue;
use std::convert::From;

#[derive(Clone, Debug)]
pub struct AbstractTicket {
    pub id: TicketId,
    pub summary: String,
    // TODO: Find out how to get the bug description from comment#0 with Bugzilla
    pub description: Option<String>,
    pub doc_type: String,
    pub doc_text: String,
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
    pub groups: Vec<String>,
    pub public: bool,
    pub requires_doc_text: DocTextStatus,
    pub duplicates: Vec<AbstractTicket>,
}

#[derive(Clone, Debug)]
pub struct TicketId {
    pub key: String,
    pub tracker: String,
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
                tracker: "Bugzilla".to_string(),
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
                .unwrap()
                .as_str()
                .unwrap()
                .to_string(),
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
            groups: bug.groups,
            // TODO: Implement public
            public: false,
            // TODO: Implement RDT
            requires_doc_text: DocTextStatus::InProgress,
            duplicates: Vec::new(),
        }
    }
}

impl AbstractTicket {
    pub fn release_note(self) -> String {
        self.doc_text
    }
}

pub fn display_bugzilla_bug(bug: &Bug) -> String {
    let doc_text = bug
        .extra
        .get("cf_release_notes")
        .unwrap()
        .as_str()
        .unwrap()
        .to_string();
    doc_text
}

pub fn display_jira_issue(issue: &JiraIssue) -> String {
    let doc_text = issue
        .fields
        .extra
        .get("customfield_12317322")
        .unwrap()
        .as_str()
        .unwrap()
        .to_string();
    doc_text
}
