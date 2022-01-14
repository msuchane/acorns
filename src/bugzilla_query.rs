// API documentation:
// https://bugzilla.redhat.com/docs/en/html/api/core/v1/general.html

use std::collections::HashMap;

use restson::{Error as RestError, Response as RestResponse, RestClient, RestPath};
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
pub struct Response {
    pub offset: i32,
    pub limit: String,
    pub total_matches: i32,
    pub bugs: Vec<Bug>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Deserialize)]
pub struct BugzillaError {
    pub error: bool,
    pub message: String,
    pub code: i32,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Deserialize)]
pub struct Bug {
    pub op_sys: String,
    pub classification: String,
    pub id: i32,
    pub url: String,
    pub creator: String,
    pub creator_detail: User,
    pub summary: String,
    pub status: String,
    pub estimated_time: i64,
    pub target_milestone: String,
    pub cc: Vec<String>,
    pub cc_detail: Vec<User>,
    pub is_open: bool,
    pub is_creator_accessible: bool,
    pub docs_contact: String,
    pub docs_contact_detail: Option<User>,
    pub assigned_to: String,
    pub assigned_to_detail: User,
    pub resolution: String,
    pub severity: String,
    pub product: String,
    pub platform: String,
    pub last_change_time: String,
    pub remaining_time: i64,
    pub priority: String,
    pub whiteboard: String,
    pub creation_time: String,
    pub is_confirmed: bool,
    pub qa_contact: String,
    pub qa_contact_detail: Option<User>,
    pub dupe_of: Option<i32>,
    pub target_release: Vec<String>,
    pub actual_time: i64,
    pub component: Vec<String>,
    pub is_cc_accessible: bool,
    pub version: Vec<String>,
    pub keywords: Vec<String>,
    pub depends_on: Vec<i32>,
    pub blocks: Vec<i32>,
    pub see_also: Vec<String>,
    pub groups: Vec<String>,
    pub deadline: Option<String>,
    pub update_token: Option<String>,
    pub work_time: Option<i64>,
    // Not part of the default response:
    pub flags: Option<Vec<Flag>>,
    pub tags: Option<Vec<String>>,
    pub dependent_products: Option<Vec<String>>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Deserialize)]
pub struct User {
    pub email: String,
    pub id: i32,
    pub name: String,
    pub real_name: String,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Deserialize)]
pub struct Flag {
    pub id: i32,
    pub type_id: i32,
    pub creation_date: String,
    pub modification_date: String,
    pub status: String,
    pub setter: String,
    pub requestee: String,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

pub fn main(host: &str, bug: &str, api_key: &str) -> Vec<Bug> {
    let mut client = RestClient::builder().blocking(host).unwrap();
    client
        .set_header("Authorization", &format!("Bearer {}", api_key))
        .unwrap();
    // Gets a bug by ID and deserializes the JSON to data variable
    let data: RestResponse<Response> = client.get(bug).unwrap();
    let response = data.into_inner();
    println!("{:#?}", response);

    response.bugs
}

// API call with one String parameter, which is the bug ID
impl RestPath<&str> for Response {
    fn get_path(param: &str) -> Result<String, RestError> {
        Ok(format!("rest/bug?id={}", param))
    }
}
