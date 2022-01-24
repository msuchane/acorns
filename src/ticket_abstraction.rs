use bugzilla_query::Bug;
use jira_query::JiraIssue;

pub struct AbstractBug {
    pub id: TicketId,
    pub summary: String,
    pub description: String,
    pub doc_type: String,
    pub doc_text: String,
    pub docs_contact: String,
    pub release_note: String,
    pub status: String,
    pub is_open: bool,
    pub priority: String,
    pub url: String,
    pub assignee: String,
    pub components: Vec<String>,
    pub product: String,
    pub labels: Vec<String>,
    pub flags: Vec<String>,
    pub target_release: String,
    pub subsystems: Vec<String>,
    pub groups: Vec<String>,
    pub public: bool,
    pub requires_doc_text: DocTextStatus,
    pub duplicates: Vec<AbstractBug>,
}

pub struct TicketId {
    pub key: String,
    pub tracker: String,
}

pub enum DocTextStatus {
    Approved,
    InProgress,
    NoDocumentation,
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
