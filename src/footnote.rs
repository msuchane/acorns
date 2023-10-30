/*
acorns: Generate an AsciiDoc release notes document from tracking tickets.
Copyright (C) 2023  Marek Suchánek  <msuchane@redhat.com>

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

//! This module provides functionality that connects to the signature format in the release note,
//! based on an optional footnote found in the manual AsciiDoc files in the docs repo.
//! 
//! If any manual AsciiDoc file defines the `PrivateTicketFootnote` footnote, private tickets
//! will add the footnote to the non-clickable ticket signature.

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


/// Search the AsciiDoc files in the RN project and see if any of them defines
/// the `PrivateTicketFootnote` footnote. Return `true` if the footnote is defined.
#[must_use]
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

/// Estimate if the given file is an AsciiDoc file.
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

/// Return `true` if the given file contains the footnote defined
/// in the `FOOTNOTE_ATTR_REGEX` regular expression.
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
