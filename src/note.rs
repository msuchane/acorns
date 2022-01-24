use crate::bugzilla_query::Bug;
use crate::jira_query::JiraIssue;

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
