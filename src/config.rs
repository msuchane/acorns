use std::fs;
use std::path::{Path, PathBuf};

use color_eyre::eyre::{bail, Context, Result};
use serde::Deserialize;

/// The name of this program, as specified in Cargo.toml. Used later to access configuration files.
const PROGRAM_NAME: &str = env!("CARGO_PKG_NAME");

/// The sub-directory inside the release notes project that contains all Cizrna configuration and other files.
/// The name of this sub-directory is the same as the name of this program.
const DATA_PREFIX: &str = PROGRAM_NAME;

// TODO: Make the output configurable. Enable saving to a separate Git repository.
/// The sub-directory inside the data directory that contains all generated documents.
const GENERATED_PREFIX: &str = "generated";

/// A request to query for a ticket in a tracker.
#[derive(Debug, Eq, PartialEq, Hash, Deserialize)]
pub struct TicketQuery {
    pub tracker: tracker::Service,
    pub key: String,
}

pub mod tracker {
    use serde::Deserialize;
    use std::fmt;

    /// An issue-tracking service, as in the platform.
    #[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Deserialize)]
    pub enum Service {
        Bugzilla,
        Jira,
    }

    impl fmt::Display for Service {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            let name = match self {
                Self::Bugzilla => "Bugzilla",
                Self::Jira => "Jira",
            };
            write!(f, "{}", name)
        }
    }

    #[derive(Debug, PartialEq, Deserialize)]
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
    #[derive(Debug, PartialEq, Deserialize)]
    pub struct Instance {
        pub host: String,
        pub api_key: Option<String>,
        pub fields: Fields,
    }

    /// The issue tracker instances configured in the current release notes project.
    #[derive(Debug, PartialEq, Deserialize)]
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
#[derive(Debug, PartialEq, Deserialize)]
pub struct Filter {
    pub doc_type: Option<Vec<String>>,
    pub subsystem: Option<Vec<String>>,
    pub component: Option<Vec<String>>,
}

/// Parse the specified tickets config file into the ticket queries configuration.
fn parse_tickets(tickets_file: &Path) -> Result<Vec<TicketQuery>> {
    let text =
        fs::read_to_string(tickets_file).context("Cannot read the tickets configuration file.")?;
    let config: Vec<TicketQuery> =
        serde_yaml::from_str(&text).context("Cannot parse the tickets configuration file.")?;
    log::debug!("{:#?}", config);

    Ok(config)
}

/// Parse the specified tracker file into the trackers configuration.
fn parse_trackers(trackers_file: &Path) -> Result<tracker::Config> {
    let text =
        fs::read_to_string(trackers_file).context("Cannot read the tickets configuration file.")?;
    let trackers: tracker::Config =
        serde_yaml::from_str(&text).context("Cannot parse the tickets configuration file.")?;
    log::debug!("{:#?}", trackers);

    Ok(trackers)
}

/// Parse the template configuration files into template structs, with chapter and section definitions.
fn parse_templates(template_file: &Path) -> Result<Template> {
    let text = fs::read_to_string(template_file).context("Cannot read the template file.")?;
    let templates: Template =
        serde_yaml::from_str(&text).context("Cannot parse the template file.")?;
    log::debug!("{:#?}", templates);
    Ok(templates)
}

/// Parsed input metadata that represent the configuration of a release notes project
pub struct Project {
    pub base_dir: PathBuf,
    pub generated_dir: PathBuf,
    pub tickets: Vec<TicketQuery>,
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

        let tickets = parse_tickets(&tickets_path)?;
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
