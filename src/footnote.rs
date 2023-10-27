use std::fs;
use std::path::Path;

use color_eyre::{Result, eyre::WrapErr};
use ignore::Walk;
use once_cell::sync::Lazy;
use regex::Regex;

use crate::REGEX_ERROR;

/// This regex looks for a footnote definition with the `PrivateTicketFootnote` ID.
static FOOTNOTE_ATTR_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"footnoteref:\[PrivateTicketFootnote,.+\]").expect(REGEX_ERROR));


pub fn is_footnote_defined(project: &Path) -> Result<bool> {
    for result in Walk::new(project) {
        // Each item yielded by the iterator is either a directory entry or an error.
        let dir_entry = result?;

        let file_path = dir_entry.path();
        
        if is_file_adoc(file_path) && file_contains_footnote(file_path)? {
            log::info!("The private ticket footnote is defined.");
            return Ok(true);
        }
    }

    Ok(false)
}

fn is_file_adoc(path: &Path) -> bool {
    let adoc_extensions = ["adoc", "asciidoc"];
    
    let file_ext = path.extension().and_then(|ext| ext.to_str());

    if let Some(ext) = file_ext {
        if adoc_extensions.contains(&ext) {
            return true;
        }
    }

    false
}

fn file_contains_footnote(path: &Path) -> Result<bool> {
    let text = fs::read_to_string(path)
        .wrap_err("Cannot read AsciiDoc file in the project repository.")?;

    let found_attr = text.lines().any(|line| {
        // Detect and reject basic line comments.
        !line.starts_with("//") &&
        // If any line contains the footnote attribute definition, return `true`.
        FOOTNOTE_ATTR_REGEX.is_match(line)
    });

    Ok(found_attr)
}
