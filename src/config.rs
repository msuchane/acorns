use std::fs;
use std::path::Path;

use log::debug;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Ticket {
    pub tracker: TrackerType,
    pub key: String,
}

#[derive(Debug, Deserialize)]
pub enum TrackerType {
    Bugzilla,
    Jira,
}

#[derive(Debug, Deserialize)]
pub struct Tracker {
    pub host: String,
    pub api_key: String,
}

#[derive(Debug, Deserialize)]
pub struct TrackerConfig {
    pub jira: Tracker,
    pub bugzilla: Tracker,
}

pub fn parse(config_file: &Path, trackers_file: &Path) -> (Vec<Ticket>, TrackerConfig) {
    let text = fs::read_to_string(config_file).unwrap();
    let config: Vec<Ticket> = serde_yaml::from_str(&text).unwrap();
    debug!("{:#?}", config);

    let text = fs::read_to_string(trackers_file).unwrap();
    let trackers: TrackerConfig = serde_yaml::from_str(&text).unwrap();
    debug!("{:#?}", trackers);

    (config, trackers)
}
