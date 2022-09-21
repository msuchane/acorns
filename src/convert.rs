use std::convert::TryFrom;
use std::fs;
use std::path::Path;

use color_eyre::{eyre::WrapErr, Result};
use regex::Regex;
use serde::Deserialize;
use serde_yaml;

use crate::config::{tracker, KeyOrSearch, TicketQueryEntry};

const BZ_PATTERN: &str = r"^BZ#(\d+)$";
const JIRA_PATTERN: &str = r"^JIRA:([A-Z0-9-]+)$";
const BZ_TRAC_PATTERN: &str = r"^BZ_TRAC#(\d+)$";
const JIRA_QUERY_PATTERN: &str = r"^JIRA_QUERY:(.*)$";
const BZ_QUERY_PATTERN: &str = r"^BZ_QUERY:(.*)$";
const PES_PATTERN: &str = r"^PES_QUERY:(\d+)\.(\d+)$";

#[derive(Debug, Deserialize)]
struct CornConfig {
    ids: Vec<CornEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CornEntry {
    id: String,
    overrides: Option<Overrides>,
    #[serde(default)]
    references: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Overrides {
    subsystem: Option<String>,
    component: Option<String>,
    doc_type: Option<String>,
}

pub fn convert(legacy: &Path, _new: &Path) -> Result<()> {
    let text = fs::read_to_string(legacy).wrap_err("Cannot read the legacy configuration file.")?;
    let legacy_config: CornConfig =
        serde_yaml::from_str(&text).wrap_err("Cannot parse the legacy configuration file.")?;

    println!("{:#?}", legacy_config);

    Ok(())
}

impl TryFrom<CornEntry> for TicketQueryEntry {
    type Error = color_eyre::eyre::Error;

    fn try_from(item: CornEntry) -> Result<Self> {
        let (service, key_or_search) = parse_stamp(&item.id)?;

        todo!()
    }
}

fn parse_stamp(stamp: &str) -> Result<(tracker::Service, KeyOrSearch)> {
    todo!()
}
