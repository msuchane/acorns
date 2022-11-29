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

/// A shared error message that displays if the static regular expressions
/// are invalid, and the regex library can't parse them.
const REGEX_ERROR: &str = "Invalid built-in regular expression.";

/// A regular expression that matches the Bugzilla ID format in CoRN 3.
static BZ_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^BZ#(\d+)$").expect(REGEX_ERROR));
/// A regular expression that matches the Jira ID format in CoRN 3.
static JIRA_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^JIRA:([A-Z0-9-]+)$").expect(REGEX_ERROR));
/// A regular expression that matches the Bugzilla tracker format in CoRN 3.
static BZ_TRAC_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^BZ_TRAC#(\d+)$").expect(REGEX_ERROR));
/// A regular expression that matches the Bugzilla query format in CoRN 3.
static BZ_QUERY_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^BZ_QUERY:(.*)$").expect(REGEX_ERROR));
/// A regular expression that matches the Jira query format in CoRN 3.
static JIRA_QUERY_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^JIRA_QUERY:(.*)$").expect(REGEX_ERROR));
/// A regular expression that matches the PES query format in CoRN 3.
static PES_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^PES_QUERY:(\d+)\.(\d+)$").expect(REGEX_ERROR));

/// An incomplete representation of the legacy `corn.yaml` configuration file.
/// We only care about the `ids` section here.
#[derive(Debug, Deserialize)]
struct CornConfig {
    ids: Vec<CornEntry>,
}

/// An entry in the legacy `corn.yaml` configuration file.
/// It represents a query for a ticket or a set of tickets.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct CornEntry {
    /// The `id` field stores both the tracker identification
    /// and the ticket key or query string.
    /// This information is encoded in a string that we can parse
    /// using a regular expression.
    id: String,
    overrides: Option<Overrides>,
    #[serde(default)]
    references: Vec<String>,
}

/// The overrides option for an entry in the legacy `corn.yaml` configuration file.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Overrides {
    subsystem: Option<String>,
    component: Option<String>,
    doc_type: Option<String>,
}

impl Overrides {
    /// Convert the legacy overrides into a string that conforms to the current
    /// configuration format.
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

/// Load the legacy, CoRN 3 configuration from a file and save the new,
/// converted configuration to a new file.
pub fn convert(legacy: &Path, new: &Path) -> Result<()> {
    log::info!(
        "Reading the legacy configuration file:\n\t{}",
        legacy.display()
    );

    let text = fs::read_to_string(legacy).wrap_err("Cannot read the legacy configuration file.")?;

    let new_config = convert_format(&text)?;

    log::info!("Saving the new configuration file:\n\t{}", new.display());
    fs::write(new, new_config).wrap_err("Cannot write to the new configuration file.")?;

    Ok(())
}

/// Convert a string containing the legacy configuration
/// to a string containing the new configuration.
///
/// The reason why we manually compose a string as the output,
/// rather than automatically serializing a structure of some kind,
/// is that we want the inline YaML syntax, where each entry fits
/// on one line. Serializing would get us the multi-line syntax.
fn convert_format(legacy_format: &str) -> Result<String> {
    let legacy_config: CornConfig = serde_yaml::from_str(legacy_format)
        .wrap_err("Cannot parse the legacy configuration file.")?;

    log::debug!("The legacy configuration:\n{:#?}", legacy_config);

    let new_entries: Vec<String> = legacy_config
        .ids
        .into_iter()
        .map(String::try_from)
        .collect::<Result<_>>()
        .wrap_err("Cannot parse an entry in the legacy configuration file.")?;

    let new_config = new_entries
        .into_iter()
        .map(|entry| format!("- {}", entry))
        .collect::<Vec<_>>()
        .join("\n");

    log::debug!("The new configuration:\n{:#?}", new_config);

    Ok(new_config)
}

impl TryFrom<CornEntry> for String {
    type Error = color_eyre::eyre::Error;

    /// Convert the legacy CoRN 3 entry to a string, which is
    /// a tagged YaML enum variant, holding a tuple.
    ///
    /// The string intentionally doesn't start with the `-` bullet point,
    /// so that we can use this function to process inline elements, too.
    fn try_from(item: CornEntry) -> Result<Self> {
        let (service, key_or_search) = parse_stamp(&item.id)?;

        let prefix = match key_or_search {
            KeyOrSearch::Key(key) => format!("!key [{}, {}", service.short_name(), key),
            KeyOrSearch::Search(search) => {
                format!("!search [{}, \"{}\"", service.short_name(), search)
            }
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

/// Parse the `id` field of the legacy CoRN 3 entry, and pull out
/// the tracker service and the ticket key or search query.
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
