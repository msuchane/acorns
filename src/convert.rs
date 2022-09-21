use std::fs;
use std::path::Path;

use color_eyre::{eyre::WrapErr, Result};
use regex::Regex;
use serde::Deserialize;
use serde_yaml;

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
