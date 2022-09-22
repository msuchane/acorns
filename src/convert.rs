//! This module provides conversion functionality to convert
//! from the legacy CoRN 3 `corn.yaml` configuration file format
//! to the current tickets.yaml format.

use std::convert::TryFrom;
use std::fs;
use std::path::Path;

use color_eyre::eyre::{eyre, WrapErr};
use color_eyre::Result;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::Deserialize;

use crate::config::{tracker::Service, KeyOrSearch};

const REGEX_ERROR: &str = "Invalid built-in regular expression.";

static BZ_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^BZ#(\d+)$").expect(REGEX_ERROR));
static JIRA_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^JIRA:([A-Z0-9-]+)$").expect(REGEX_ERROR));
static BZ_TRAC_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^BZ_TRAC#(\d+)$").expect(REGEX_ERROR));
static BZ_QUERY_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^BZ_QUERY:(.*)$").expect(REGEX_ERROR));
static JIRA_QUERY_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^JIRA_QUERY:(.*)$").expect(REGEX_ERROR));
static PES_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^PES_QUERY:(\d+)\.(\d+)$").expect(REGEX_ERROR));

#[derive(Debug, Deserialize)]
struct CornConfig {
    ids: Vec<CornEntry>,
}

#[derive(Debug, Default, Deserialize)]
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

impl Overrides {
    fn into_new_format(self) -> String {
        let ssts = self.subsystem.map(|sst| format!("subsystems: [{}]", sst));
        let components = self
            .component
            .map(|component| format!("components: [{}]", component));
        let doc_type = self
            .doc_type
            .map(|doc_type| format!("doc_type: {}", doc_type));

        let list = [ssts, components, doc_type]
            .into_iter()
            // Take only the `Some` variants.
            .flatten()
            .collect::<Vec<String>>()
            .join(", ");

        format!("overrides: {{{}}}", list)
    }
}

pub fn convert(legacy: &Path, _new: &Path) -> Result<()> {
    let text = fs::read_to_string(legacy).wrap_err("Cannot read the legacy configuration file.")?;
    let legacy_config: CornConfig =
        serde_yaml::from_str(&text).wrap_err("Cannot parse the legacy configuration file.")?;

    println!("{:#?}", legacy_config);

    for entry in legacy_config.ids {
        let new_entry = String::try_from(entry)?;
        println!("- {}", new_entry);
    }

    Ok(())
}

impl TryFrom<CornEntry> for String {
    type Error = color_eyre::eyre::Error;

    fn try_from(item: CornEntry) -> Result<Self> {
        let (service, key_or_search) = parse_stamp(&item.id)?;

        let prefix = match key_or_search {
            KeyOrSearch::Key(key) => format!("!key [{}, \"{}\"", service, key),
            KeyOrSearch::Search(search) => format!("!search [{}, \"{}\"", service, search),
        };

        let overrides = item.overrides.map(Overrides::into_new_format);

        let references: Vec<String> = item
            .references
            .into_iter()
            .map(|reference| {
                let legacy_entry = CornEntry {
                    id: reference,
                    ..Default::default()
                };
                String::try_from(legacy_entry)
            })
            .collect::<Result<_>>()?;

        let references = if references.is_empty() {
            None
        } else {
            Some(format!("references: [{}]", references.join(", ")))
        };

        let options = if overrides.is_some() || references.is_some() {
            let list = [overrides, references]
                .into_iter()
                .flatten()
                .collect::<Vec<String>>()
                .join(", ");
            Some(format!("{{ {} }}", list))
        } else {
            None
        };

        let new_entry = [Some(prefix), options]
            .into_iter()
            // Take only the `Some` variants.
            .flatten()
            .collect::<Vec<String>>()
            .join(", ");

        Ok(new_entry + "]")
    }
}

fn parse_stamp(stamp: &str) -> Result<(Service, KeyOrSearch)> {
    // Supported options
    if let Some(captures) = BZ_REGEX.captures(stamp) {
        let service = Service::Bugzilla;
        let key = KeyOrSearch::Key(captures[1].to_string());
        Ok((service, key))
    } else if let Some(captures) = JIRA_REGEX.captures(stamp) {
        let service = Service::Jira;
        let key = KeyOrSearch::Key(captures[1].to_string());
        Ok((service, key))
    } else if let Some(captures) = BZ_QUERY_REGEX.captures(stamp) {
        let service = Service::Bugzilla;
        let search = KeyOrSearch::Search(captures[1].to_string());
        Ok((service, search))
    } else if let Some(captures) = JIRA_QUERY_REGEX.captures(stamp) {
        let service = Service::Jira;
        let search = KeyOrSearch::Search(captures[1].to_string());
        Ok((service, search))
    // Unsupported options
    } else if BZ_TRAC_REGEX.is_match(stamp) {
        Err(eyre!("The Bugzilla tracker option is not implemented yet."))
    } else if PES_REGEX.is_match(stamp) {
        Err(eyre!("The PES option is not implemented yet."))
    } else {
        Err(eyre!("Failed to parse the ticket ID: `{}`", stamp))
    }
}
