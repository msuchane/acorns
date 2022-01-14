// API documentation:
// https://bugzilla.redhat.com/docs/en/html/api/core/v1/general.html

use std::collections::HashMap;

use restson::{Error, Response, RestClient, RestPath};
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
struct BZResponse {
    offset: i32,
    limit: String,
    total_matches: i32,
    bugs: Vec<Bug>,
    #[serde(flatten)]
    extra: HashMap<String, Value>,
}

#[derive(Debug, Deserialize)]
struct BugzillaError {
    error: bool,
    message: String,
    code: i32,
    #[serde(flatten)]
    extra: HashMap<String, Value>,
}

#[derive(Debug, Deserialize)]
struct Bug {
    op_sys: String,
    classification: String,
    id: i32,
    url: String,
    creator: String,
    creator_detail: User,
    summary: String,
    status: String,
    estimated_time: i64,
    target_milestone: String,
    cc: Vec<String>,
    cc_detail: Vec<User>,
    is_open: bool,
    is_creator_accessible: bool,
    docs_contact: String,
    docs_contact_detail: Option<User>,
    assigned_to: String,
    assigned_to_detail: User,
    resolution: String,
    severity: String,
    product: String,
    platform: String,
    last_change_time: String,
    remaining_time: i64,
    priority: String,
    whiteboard: String,
    creation_time: String,
    is_confirmed: bool,
    qa_contact: String,
    qa_contact_detail: Option<User>,
    dupe_of: Option<i32>,
    target_release: Vec<String>,
    actual_time: i64,
    component: Vec<String>,
    is_cc_accessible: bool,
    version: Vec<String>,
    keywords: Vec<String>,
    depends_on: Vec<i32>,
    blocks: Vec<i32>,
    see_also: Vec<String>,
    groups: Vec<String>,
    deadline: Option<String>,
    update_token: Option<String>,
    work_time: Option<i64>,
    // Not part of the default response:
    flags: Option<Vec<Flag>>,
    tags: Option<Vec<String>>,
    dependent_products: Option<Vec<String>>,
    #[serde(flatten)]
    extra: HashMap<String, Value>,
}

#[derive(Debug, Deserialize)]
struct User {
    email: String,
    id: i32,
    name: String,
    real_name: String,
    #[serde(flatten)]
    extra: HashMap<String, Value>,
}

#[derive(Debug, Deserialize)]
struct Flag {
    id: i32,
    type_id: i32,
    creation_date: String,
    modification_date: String,
    status: String,
    setter: String,
    requestee: String,
    #[serde(flatten)]
    extra: HashMap<String, Value>,
}

pub fn main(host: &str, bug: &str, api_key: &str) {
    let mut client = RestClient::builder().blocking(host).unwrap();
    client
        .set_header("Authorization", &format!("Bearer {}", api_key))
        .unwrap();
    // Gets a bug by ID and deserializes the JSON to data variable
    let data: Response<BZResponse> = client.get(bug).unwrap();
    println!("{:#?}", data.into_inner());

    // println!("{:#?}", data);
}

// API call with one String parameter, which is the bug ID
impl RestPath<&str> for BZResponse {
    fn get_path(param: &str) -> Result<String, Error> {
        Ok(format!("rest/bug?id={}", param))
    }
}
