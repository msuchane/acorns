use std::collections::HashMap;

use restson::{Error, Response, RestClient, RestPath};
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
pub struct JiraIssue {
    id: String,
    key: String,
    expand: String,
    fields: Fields,
    #[serde(rename = "self")]
    self_link: String,
    #[serde(flatten)]
    extra: HashMap<String, Value>,
}

#[derive(Debug, Deserialize)]
pub struct Fields {
    #[serde(rename = "lastViewed")]
    last_viewed: Option<String>,
    labels: Vec<String>,
    versions: Vec<Version>,
    assignee: User,
    description: Option<String>,
    duedate: Option<String>,
    #[serde(rename = "fixVersions")]
    fix_versions: Vec<Version>,
    reporter: User,
    status: Status,
    created: String,
    updated: String,
    issuetype: IssueType,
    timeestimate: Option<i32>,
    aggregatetimeestimate: Option<i32>,
    timeoriginalestimate: Option<i32>,
    timespent: Option<i32>,
    aggregatetimespent: Option<i32>,
    aggregatetimeoriginalestimate: Option<i32>,
    progress: Progress,
    aggregateprogress: Progress,
    workratio: i32,
    summary: String,
    creator: User,
    project: Project,
    priority: Priority,
    components: Vec<Component>,
    watches: Watches,
    archiveddate: Option<String>,
    archivedby: Option<String>,
    resolution: Option<Resolution>,
    resolutiondate: Option<String>,
    comment: Comments,
    issuelinks: Vec<IssueLink>,
    votes: Votes,
    parent: Option<Parent>,
    #[serde(flatten)]
    extra: HashMap<String, Value>,
}

#[derive(Debug, Deserialize)]
pub struct User {
    active: bool,
    #[serde(rename = "displayName")]
    display_name: String,
    #[serde(rename = "emailAddress")]
    email_address: String,
    key: String,
    name: String,
    #[serde(rename = "timeZone")]
    time_zone: String,
    #[serde(rename = "avatarUrls")]
    avatar_urls: AvatarUrls,
    #[serde(rename = "self")]
    self_link: String,
    #[serde(flatten)]
    extra: HashMap<String, Value>,
}

#[derive(Debug, Deserialize)]
pub struct Version {
    id: String,
    description: Option<String>,
    name: String,
    archived: bool,
    released: bool,
    #[serde(rename = "releaseDate")]
    release_date: Option<String>,
    #[serde(flatten)]
    extra: HashMap<String, Value>,
}

#[derive(Debug, Deserialize)]
pub struct Status {
    description: String,
    #[serde(rename = "iconUrl")]
    icon_url: String,
    id: String,
    name: String,
    #[serde(rename = "statusCategory")]
    status_category: StatusCategory,
    #[serde(rename = "self")]
    self_link: String,
    #[serde(flatten)]
    extra: HashMap<String, Value>,
}

#[derive(Debug, Deserialize)]
pub struct StatusCategory {
    #[serde(rename = "colorName")]
    color_name: String,
    id: i32,
    key: String,
    name: String,
    #[serde(rename = "self")]
    self_link: String,
    #[serde(flatten)]
    extra: HashMap<String, Value>,
}

#[derive(Debug, Deserialize)]
pub struct Resolution {
    description: String,
    id: String,
    name: String,
    #[serde(flatten)]
    extra: HashMap<String, Value>,
}

#[derive(Debug, Deserialize)]
pub struct IssueType {
    #[serde(rename = "avatarId")]
    avatar_id: i32,
    description: String,
    #[serde(rename = "iconUrl")]
    icon_url: String,
    id: String,
    name: String,
    subtask: bool,
    #[serde(rename = "self")]
    self_link: String,
    #[serde(flatten)]
    extra: HashMap<String, Value>,
}

#[derive(Debug, Deserialize)]
pub struct Project {
    id: String,
    key: String,
    name: String,
    #[serde(rename = "projectTypeKey")]
    project_type_key: String,
    #[serde(rename = "projectCategory")]
    project_category: ProjectCategory,
    #[serde(rename = "avatarUrls")]
    avatar_urls: AvatarUrls,
    #[serde(rename = "self")]
    self_link: String,
    #[serde(flatten)]
    extra: HashMap<String, Value>,
}

#[derive(Debug, Deserialize)]
pub struct ProjectCategory {
    description: String,
    id: String,
    name: String,
    #[serde(rename = "self")]
    self_link: String,
    #[serde(flatten)]
    extra: HashMap<String, Value>,
}

#[derive(Debug, Deserialize)]
pub struct Priority {
    #[serde(rename = "iconUrl")]
    icon_url: String,
    id: String,
    name: String,
    #[serde(rename = "self")]
    self_link: String,
    #[serde(flatten)]
    extra: HashMap<String, Value>,
}

#[derive(Debug, Deserialize)]
pub struct Component {
    description: Option<String>,
    id: String,
    name: String,
    #[serde(flatten)]
    extra: HashMap<String, Value>,
}

#[derive(Debug, Deserialize)]
pub struct Watches {
    #[serde(rename = "isWatching")]
    is_watching: bool,
    #[serde(rename = "watchCount")]
    watch_count: i32,
    #[serde(rename = "self")]
    self_link: String,
    #[serde(flatten)]
    extra: HashMap<String, Value>,
}

#[derive(Debug, Deserialize)]
pub struct Progress {
    progress: i32,
    total: i32,
    #[serde(flatten)]
    extra: HashMap<String, Value>,
}

#[derive(Debug, Deserialize)]
pub struct Comment {
    author: User,
    body: String,
    created: String,
    id: String,
    #[serde(rename = "updateAuthor")]
    update_author: User,
    updated: String,
    #[serde(rename = "self")]
    self_link: String,
    #[serde(flatten)]
    extra: HashMap<String, Value>,
}

#[derive(Debug, Deserialize)]
pub struct Comments {
    comments: Vec<Comment>,
    #[serde(rename = "maxResults")]
    max_results: i32,
    #[serde(rename = "startAt")]
    start_at: i32,
    total: i32,
    #[serde(flatten)]
    extra: HashMap<String, Value>,
}

#[derive(Debug, Deserialize)]
pub struct IssueLink {
    id: String,
    #[serde(rename = "outwardIssue")]
    outward_issue: OutwardIssue,
    #[serde(rename = "type")]
    link_type: IssueLinkType,
    #[serde(rename = "self")]
    self_link: String,
    #[serde(flatten)]
    extra: HashMap<String, Value>,
}

#[derive(Debug, Deserialize)]
pub struct OutwardIssue {
    id: String,
    key: String,
    fields: OutwardIssueFields,
    #[serde(rename = "self")]
    self_link: String,
    #[serde(flatten)]
    extra: HashMap<String, Value>,
}

#[derive(Debug, Deserialize)]
pub struct OutwardIssueFields {
    issuetype: IssueType,
    priority: Option<Priority>,
    status: Status,
    summary: String,
    #[serde(flatten)]
    extra: HashMap<String, Value>,
}

#[derive(Debug, Deserialize)]
pub struct IssueLinkType {
    id: String,
    inward: String,
    name: String,
    outward: String,
    #[serde(rename = "self")]
    self_link: String,
    #[serde(flatten)]
    extra: HashMap<String, Value>,
}

#[derive(Debug, Deserialize)]
pub struct Votes {
    #[serde(rename = "hasVoted")]
    has_voted: bool,
    votes: i32,
    #[serde(rename = "self")]
    self_link: String,
    #[serde(flatten)]
    extra: HashMap<String, Value>,
}

#[derive(Debug, Deserialize)]
pub struct AvatarUrls {
    #[serde(rename = "16x16")]
    xsmall: String,
    #[serde(rename = "24x24")]
    small: String,
    #[serde(rename = "32x32")]
    medium: String,
    #[serde(rename = "48x48")]
    full: String,
    #[serde(flatten)]
    extra: HashMap<String, Value>,
}

#[derive(Debug, Deserialize)]
pub struct Parent {
    fields: ParentFields,
    id: String,
    key: String,
    #[serde(rename = "self")]
    self_link: String,
    #[serde(flatten)]
    extra: HashMap<String, Value>,
}

#[derive(Debug, Deserialize)]
pub struct ParentFields {
    issuetype: IssueType,
    priority: Priority,
    status: Status,
    summary: String,
    #[serde(flatten)]
    extra: HashMap<String, Value>,
}

pub fn main(host: &str, issue: &str, api_key: &str) {
    let mut client = RestClient::builder().blocking(host).unwrap();
    client
        .set_header("Authorization", &format!("Bearer {}", api_key))
        .unwrap();
    // Gets a bug by ID and deserializes the JSON to data variable
    let data: Response<JiraIssue> = client.get(issue).unwrap();
    println!("{:#?}", data.into_inner());

    // println!("{:#?}", data);
}

// API call with one String parameter (e.g. "https://issues.redhat.com/rest/api/2/issue/RHELPLAN-12345")
impl RestPath<&str> for JiraIssue {
    fn get_path(param: &str) -> Result<String, Error> {
        Ok(format!("rest/api/2/issue/{}", param))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
