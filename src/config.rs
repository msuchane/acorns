/*
cizrna: Generate an AsciiDoc release notes document from tracking tickets.
Copyright (C) 2022  Marek Suchánek  <msuchane@redhat.com>

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

use std::convert::From;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use color_eyre::eyre::{bail, Result, WrapErr};
use serde::Deserialize;

/// The name of this program, as specified in Cargo.toml. Used later to access configuration files.
const PROGRAM_NAME: &str = env!("CARGO_PKG_NAME");

/// The sub-directory inside the release notes project that contains all Cizrna configuration and other files.
/// The name of this sub-directory is the same as the name of this program.
const DATA_PREFIX: &str = PROGRAM_NAME;

// TODO: Make the output configurable. Enable saving to a separate Git repository.
/// The sub-directory inside the data directory that contains all generated documents.
const GENERATED_PREFIX: &str = "generated";

/// A ticket query extracted from the user configuration file.
/// It holds all the information necessary to download information
/// on a particular ticket or a group of tickets from an issue tracker.
#[derive(Debug, Eq, PartialEq, Hash)]
pub struct TicketQuery {
    pub tracker: tracker::Service,
    pub using: KeyOrSearch,
    pub overrides: Option<Overrides>,
    pub references: Vec<Arc<TicketQuery>>,
}

/// Variants of the ticket query that the user can configure in `tickets.yaml`.
///
/// * `Key`: Requests a specific ticket by its key.
/// * `Free`: Requests all tickets that match a free-form query.
#[derive(Debug, Eq, PartialEq, Hash, Deserialize)]
pub enum KeyOrSearch {
    Key(String),
    Search(String),
}

/// A ticket query as defined in the user configuration file.
/// This entry struct is separate from `TicketQuery` because
/// this tuple format is more ergonomic to write in config files,
/// and it enables us to wrap references in `Arc` when converting
/// from this struct to `TicketQuery`.
/// Otherwise, `Arc` doesn't implement `Deserialize`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TicketQueryEntry(
    tracker::Service,
    Identifier,
    #[serde(default)] TicketQueryOptions,
);

impl From<TicketQueryEntry> for TicketQuery {
    fn from(item: TicketQueryEntry) -> Self {
        // Destructure all the parts of the query to avoid trouble with partial moves
        // and to avoid cloning.
        let (tracker, identifier, options) = (item.0, item.1, item.2);
        let references: Vec<Arc<TicketQuery>> = options
            .references
            .into_iter()
            .map(Self::from)
            .map(Arc::new)
            .collect();

        Self {
            using: identifier.into(),
            tracker,
            overrides: options.overrides,
            references,
        }
    }
}

/// The string that identifies tickets to pull from the tracker,
/// either in the form of a ticket key (which can be a string or a number),
/// or in the form of a search string.
///
/// This is practically an enum. The later processing of this struct rejects
/// variants where both or none of the fields are `Some`.
/// However, using an actual enum would cause problems with teh YaML representation
/// in the configuration file, because serde_yaml distinguishes variants using tags,
/// which aren't well supported in editors. Therefore, this struct emulates an enum
/// and provides a readable YaML syntax.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Identifier {
    key: Option<KeyFormats>,
    search: Option<String>,
}

impl From<Identifier> for KeyOrSearch {
    fn from(item: Identifier) -> Self {
        match (item.key.clone(), item.search.clone()) {
            (Some(key), None) => KeyOrSearch::Key(key.into_string()),
            (None, Some(search)) => KeyOrSearch::Search(search),
            (Some(_), Some(_)) => panic!("Please specify only one entry:\n{item:#?}"),
            (None, None) => panic!("Please specify at least one entry:\n{item:#?}"),
        }
    }
}

/// A simple enum between a string and an integer.
///
/// This increases ergonomics in specifying the tickets in the configuration file,
/// because you can specify Bugzilla keys as numbers without any quotes, such as `[BZ, 12345]`.
#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
enum KeyFormats {
    String(String),
    Number(i32),
}

impl KeyFormats {
    /// Convert the enum to a string:
    /// either returns the string variant as is, or stringify the integer.
    fn into_string(self) -> String {
        match self {
            Self::String(s) => s,
            Self::Number(n) => n.to_string(),
        }
    }
}

/// A shared options entry in a ticket query written
/// in the configuration file enum format.
#[derive(Debug, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct TicketQueryOptions {
    overrides: Option<Overrides>,
    references: Vec<TicketQueryEntry>,
}

/// Optional, configurable overrides that modify an `AbstractTicket`.
/// The selected fields that you can modify affect the sorting of the ticket in the document.
#[derive(Debug, Eq, PartialEq, Hash, Deserialize)]
pub struct Overrides {
    pub doc_type: Option<String>,
    pub components: Option<Vec<String>>,
    pub subsystems: Option<Vec<String>>,
}

pub mod tracker {
    use serde::{Deserialize, Serialize};
    use std::fmt;

    /// An issue-tracking service, as in the platform.
    #[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
    pub enum Service {
        #[serde(alias = "BZ")]
        Bugzilla,
        Jira,
    }

    impl fmt::Display for Service {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            let name = match self {
                Self::Bugzilla => "Bugzilla",
                Self::Jira => "Jira",
            };
            write!(f, "{name}")
        }
    }

    impl Service {
        /// Return the short name or acronym of the service, if any.
        /// Otherwise, return the regular name.
        pub fn short_name(self) -> &'static str {
            match self {
                Self::Bugzilla => "BZ",
                Self::Jira => "Jira",
            }
        }
    }

    #[derive(Debug, Eq, PartialEq, Deserialize)]
    pub struct Fields {
        pub doc_type: String,
        pub doc_text: String,
        pub doc_text_status: String,
        pub docs_contact: String,
        pub target_release: String,
        pub subsystems: String,
    }

    /// The particular instance of an issue tracker,
    /// with a host URL and access credentials.
    #[derive(Debug, Eq, PartialEq, Deserialize)]
    pub struct Instance {
        pub host: String,
        pub api_key: Option<String>,
        pub fields: Fields,
    }

    /// The issue tracker instances configured in the current release notes project.
    #[derive(Debug, Eq, PartialEq, Deserialize)]
    pub struct Config {
        pub jira: Instance,
        pub bugzilla: Instance,
    }
}

/// This struct models the template configuration file.
/// It includes both `chapters` and `sections` because this is a way
/// in YaML to create reusable section definitions that can then
/// appear several times in different places. They have to be defined
/// on the top level, outside the actual chapters.
#[derive(Debug, PartialEq, Deserialize)]
pub struct Template {
    pub chapters: Vec<Section>,
    pub sections: Option<Vec<Section>>,
}

/// This struct covers the necessary properties of a section, which can either
/// turn into a module if it's a leaf, or into an assembly if it includes
/// more sections.
///
/// The `filter` field narrows down the tickets that can appear in this module
/// or in the modules that are included in this assembly.
#[derive(Debug, PartialEq, Deserialize)]
pub struct Section {
    pub title: String,
    pub intro_abstract: Option<String>,
    pub filter: Filter,
    pub sections: Option<Vec<Section>>,
}

/// The configuration of a filter, which narrows down the tickets
/// that can appear in the section that the filter belongs to.
#[derive(Debug, Eq, PartialEq, Deserialize)]
pub struct Filter {
    pub doc_type: Option<Vec<String>>,
    pub subsystem: Option<Vec<String>>,
    pub component: Option<Vec<String>>,
}

/// Parse the specified tickets config file into the ticket queries configuration.
fn parse_tickets(tickets_file: &Path) -> Result<Vec<TicketQuery>> {
    let text =
        fs::read_to_string(tickets_file).wrap_err("Cannot read the tickets configuration file.")?;
    let config: Vec<TicketQueryEntry> =
        serde_yaml::from_str(&text).wrap_err("Cannot parse the tickets configuration file.")?;
    log::debug!("{:#?}", config);

    let queries = config.into_iter().map(TicketQuery::from).collect();

    Ok(queries)
}

/// Parse the specified tracker file into the trackers configuration.
fn parse_trackers(trackers_file: &Path) -> Result<tracker::Config> {
    let text = fs::read_to_string(trackers_file)
        .wrap_err("Cannot read the tickets configuration file.")?;
    let trackers: tracker::Config =
        serde_yaml::from_str(&text).wrap_err("Cannot parse the tickets configuration file.")?;
    log::debug!("{:#?}", trackers);

    Ok(trackers)
}

/// Parse the template configuration files into template structs, with chapter and section definitions.
fn parse_templates(template_file: &Path) -> Result<Template> {
    let text = fs::read_to_string(template_file).wrap_err("Cannot read the template file.")?;
    let templates: Template =
        serde_yaml::from_str(&text).wrap_err("Cannot parse the template file.")?;
    log::debug!("{:#?}", templates);
    Ok(templates)
}

/// Parsed input metadata that represent the configuration of a release notes project
pub struct Project {
    pub base_dir: PathBuf,
    pub generated_dir: PathBuf,
    pub tickets: Vec<Arc<TicketQuery>>,
    pub trackers: tracker::Config,
    pub templates: Template,
}

impl Project {
    /// Set up a Project configuration, including parsed configuration files
    /// and paths to the relevant project directories.
    pub fn new(directory: &Path) -> Result<Self> {
        let abs_path = directory.canonicalize()?;
        let data_dir = abs_path.join(DATA_PREFIX);
        let generated_dir = data_dir.join(GENERATED_PREFIX);

        // If not even the main configuration directory exists, exit with an error.
        if !data_dir.is_dir() {
            bail!(
                "The configuration directory is missing: {}",
                data_dir.display()
            );
        }

        // Prepare to access each configuration file.
        // TODO: Possibly enable overriding the default config paths.
        let tickets_path = data_dir.join("tickets.yaml");
        let trackers_path = data_dir.join("trackers.yaml");
        let templates_path = data_dir.join("templates.yaml");

        log::debug!(
            "Configuration files:\n* {}\n* {}\n* {}",
            tickets_path.display(),
            trackers_path.display(),
            templates_path.display()
        );

        let tickets = parse_tickets(&tickets_path)?
            .into_iter()
            .map(Arc::new)
            .collect();
        let trackers = parse_trackers(&trackers_path)?;
        let templates = parse_templates(&templates_path)?;

        Ok(Self {
            base_dir: abs_path,
            generated_dir,
            tickets,
            trackers,
            templates,
        })
    }
}
