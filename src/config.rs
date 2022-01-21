use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct Ticket {
    pub tracker: TrackerType,
    pub key: String,
}

#[derive(Debug, Deserialize)]
pub enum TrackerType {
    Bugzilla,
    JIRA,
}

#[derive(Debug, Deserialize)]
pub struct Tracker {
    tool: TrackerType,
    host: String,
    api_key: String,
}

pub fn get(config_file: &Path, trackers_file: &Path) -> (Vec<Ticket>, Vec<Tracker>) {
    let text = fs::read_to_string(config_file).unwrap();
    let config: Vec<Ticket> = serde_yaml::from_str(&text).unwrap();
    println!("{:#?}", config);

    let text = fs::read_to_string(trackers_file).unwrap();
    let trackers: Vec<Tracker> = serde_yaml::from_str(&text).unwrap();
    println!("{:#?}", trackers);

    (config, trackers)
}
