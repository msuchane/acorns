use std::fs;
use std::path::Path;

use color_eyre::eyre::{Context, Result};
use log::debug;
use serde::Deserialize;

#[derive(Debug, Eq, PartialEq, Hash, Deserialize)]
pub struct TicketQuery {
    pub tracker: tracker::Service,
    pub key: String,
}

pub mod tracker {
    use serde::Deserialize;

    #[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize)]
    pub enum Service {
        Bugzilla,
        Jira,
    }

    #[derive(Debug, PartialEq, Deserialize)]
    pub struct Instance {
        pub host: String,
        pub api_key: String,
    }

    #[derive(Debug, PartialEq, Deserialize)]
    pub struct Config {
        pub jira: Instance,
        pub bugzilla: Instance,
    }
}

pub fn parse(
    config_file: &Path,
    trackers_file: &Path,
) -> Result<(Vec<TicketQuery>, tracker::Config)> {
    let text =
        fs::read_to_string(config_file).context("Cannot read the tickets configuration file.")?;
    let config: Vec<TicketQuery> =
        serde_yaml::from_str(&text).context("Cannot parse the tickets configuration file.")?;
    debug!("{:#?}", config);

    let text =
        fs::read_to_string(trackers_file).context("Cannot read the tickets configuration file.")?;
    let trackers: tracker::Config =
        serde_yaml::from_str(&text).context("Cannot parse the tickets configuration file.")?;
    debug!("{:#?}", trackers);

    Ok((config, trackers))
}
