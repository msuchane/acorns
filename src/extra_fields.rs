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

use std::convert::TryFrom;
use std::fmt;
use std::string::ToString;

use color_eyre::{
    eyre::{bail, eyre, WrapErr},
    Result,
};
use serde::Deserialize;
use serde_json::value::Value;

use bugzilla_query::Bug;
use jira_query::Issue;

use crate::config::tracker;

/// The status or progress of the release note.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DocTextStatus {
    Approved,
    InProgress,
    NoDocumentation,
}

impl TryFrom<&str> for DocTextStatus {
    type Error = color_eyre::eyre::Error;

    fn try_from(string: &str) -> Result<Self> {
        match string {
            "+" | "Done" => Ok(Self::Approved),
            "?" | "Proposed" | "In progress" | "Unset" => Ok(Self::InProgress),
            // TODO: Does "Upstream only" really mean to skip this RN?
            "-" | "Rejected" | "Upstream only" => Ok(Self::NoDocumentation),
            _ => bail!("Unrecognized doc text status value: {:?}", string),
        }
    }
}

impl fmt::Display for DocTextStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let display = match self {
            Self::Approved => "Done",
            Self::InProgress => "WIP",
            Self::NoDocumentation => "No docs",
        };
        write!(f, "{display}")
    }
}

/// A wrapper around `Option<String>` that stores the docs contact email address.
///
/// On top of `Option`, this wrapper implements the `Display` trait:
///
/// * If the docs contact is `Some(String)`, the wrapper displays the string,
///   unless the string is empty, in which case it reverts to a placeholder.
/// * If the docs contact is `None`, the wrapper displays a placeholder.
#[derive(Clone, Debug)]
pub struct DocsContact(pub Option<String>);

impl fmt::Display for DocsContact {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let display = self.as_str();
        write!(f, "{display}")
    }
}

impl DocsContact {
    /// Provide the docs contact as a string slice, either of the actual docs contact,
    /// or a slice of a place holder if the docs contact is empty.
    ///
    /// This slice method is useful as a way to avoid the complete `.to_string` method,
    /// and to get a slice owned by this struct itself.
    pub fn as_str(&self) -> &str {
        let placeholder = "Missing docs contact";

        match &self.0 {
            Some(text) => {
                if text.is_empty() {
                    placeholder
                } else {
                    text
                }
            }
            None => placeholder,
        }
    }
}

pub trait ExtraFields {
    /// Extract the doc type from the ticket.
    fn doc_type(&self, config: &tracker::Fields) -> Result<String>;
    /// Extract the doc text from the ticket.
    fn doc_text(&self, config: &tracker::Fields) -> Result<String>;
    /// Extract the target release from the ticket.
    fn target_releases(&self, config: &tracker::Fields) -> Result<Vec<String>>;
    /// Extract the subsystems from the ticket.
    fn subsystems(&self, config: &tracker::Fields) -> Result<Vec<String>>;
    /// Extract the doc text status ("requires doc text") from the ticket.
    fn doc_text_status(&self, config: &tracker::Fields) -> Result<DocTextStatus>;
    /// Extract the docs contact from the ticket.
    fn docs_contact(&self, config: &tracker::Fields) -> DocsContact;
    /// Construct a URL back to the original ticket online.
    fn url(&self, tracker: &tracker::Instance) -> String;
}

#[derive(Deserialize, Debug)]
struct BzPool {
    team: BzTeam,
}

#[derive(Deserialize, Debug)]
struct BzTeam {
    name: String,
}

/// A helper function to handle and report errors when extracting a string value
/// from a custom Bugzilla or Jira field.
///
/// Returns an error is the field is missing or if it is not a string.
fn extract_field(extra: &Value, field: &str, id: impl fmt::Display) -> Result<String> {
    let field_value = extra.get(field);

    // This check covers the case where the field exists, but its value
    // is unset. I think it's safe to treat it as an empty string.
    if let Some(Value::Null) = field_value {
        log::warn!("Field {} is unset in ticket {}.", field, id);
        return Ok(String::new());
    }

    field_value
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| {
            eyre!(
                "The `{}` field is missing or has an unexpected structure:\n{:#?}",
                field,
                extra.get(field)
            )
        })
}

impl ExtraFields for Bug {
    fn doc_type(&self, config: &tracker::Fields) -> Result<String> {
        let field = &config.doc_type[0];
        extract_field(&self.extra, field, self.id)
            .wrap_err_with(|| eyre!("Failed to extract the doc type of bug {}.", self.id))
    }

    fn doc_text(&self, config: &tracker::Fields) -> Result<String> {
        let field = &config.doc_text[0];
        extract_field(&self.extra, field, self.id)
            .wrap_err_with(|| eyre!("Failed to extract the doc text of bug {}.", self.id))
    }

    fn target_releases(&self, config: &tracker::Fields) -> Result<Vec<String>> {
        let field = &config.target_release[0];
        let release = if let Ok(release) = extract_field(&self.extra, field, self.id) {
            release
        } else {
            // The target release field isn't critical. Log the problem
            // and return an empty list of releases.
            log::warn!("Failed to extract the target release of bug {}.", self.id);
            return Ok(vec![]);
        };

        // Bugzilla uses the "---" placeholder to represent an unset release.
        // TODO: Are there any more placeholder?
        let empty_values = ["---"];

        // If the release is unset, return no releases. If it's set, return that one release.
        if empty_values.contains(&release.as_str()) {
            Ok(vec![])
        } else {
            Ok(vec![release])
        }
    }

    fn subsystems(&self, config: &tracker::Fields) -> Result<Vec<String>> {
        let field = &config.subsystems[0];
        let pool_field = self
            .extra
            .get(field)
            .ok_or_else(|| eyre!("Field {} is missing.", field))?;
        let pool: BzPool = serde_json::from_value(pool_field.clone()).wrap_err_with(|| {
            eyre!(
                "The pool field has an unexpected structure in bug {}:\n{:#?}",
                self.id,
                pool_field
            )
        })?;

        // In Bugzilla, the bug always has just one subsystem. Therefore,
        // this returns a vector with a single item, or an empty vector.
        Ok(vec![pool.team.name])
    }

    fn doc_text_status(&self, config: &tracker::Fields) -> Result<DocTextStatus> {
        // If the RDT flag is unset, use this:
        let default_rdt = "?";

        let flag = &config.doc_text_status[0];

        // If the flag is unset, treat it only as a warning, not a breaking error,
        // and proceed with the default value.
        // An unset RDT is a relatively common occurence on Bugzilla.
        let rdt = self.get_flag(flag).unwrap_or_else(|| {
            log::warn!("The `{}` flag is missing in bug {}.", flag, self.id);
            default_rdt
        });

        DocTextStatus::try_from(rdt)
            .wrap_err_with(|| eyre!("Failed to extract the doc text status of bug {}.", self.id))
    }

    fn docs_contact(&self, _config: &tracker::Fields) -> DocsContact {
        if self.docs_contact.is_none() {
            log::warn!("The `docs_contact` field is missing in bug {}.", self.id);
        }

        // TODO: There's probably a way to avoid this clone.
        DocsContact(self.docs_contact.clone())
    }

    fn url(&self, tracker: &tracker::Instance) -> String {
        format!("{}/show_bug.cgi?id={}", tracker.host, &self.id)
    }
}

#[derive(Deserialize, Debug)]
struct JiraDocType {
    value: String,
}

#[derive(Deserialize, Debug)]
struct JiraSST {
    value: String,
}

impl ExtraFields for Issue {
    fn doc_type(&self, config: &tracker::Fields) -> Result<String> {
        let field = &config.doc_type[0];
        let doc_type_field = self
            .fields
            .extra
            .get(field)
            .ok_or_else(|| eyre!("The `{}` field is missing in issue {}.", field, self.key))?;
        let doc_type: JiraDocType =
            serde_json::from_value(doc_type_field.clone()).wrap_err_with(|| {
                eyre!(
                    "The doc type field has an unexpected structure in issue {}:\n{:#?}",
                    self.key,
                    doc_type_field
                )
            })?;

        Ok(doc_type.value)
    }

    fn doc_text(&self, config: &tracker::Fields) -> Result<String> {
        let field = &config.doc_text[0];
        extract_field(&self.fields.extra, field, &self.key)
            .wrap_err_with(|| eyre!("Failed to extract the doc text of issue {}.", &self.key))
    }

    fn target_releases(&self, _config: &tracker::Fields) -> Result<Vec<String>> {
        Ok(self
            .fields
            .fix_versions
            .iter()
            // TODO: Get rid of the clone if possible
            .map(|version| version.name.clone())
            .collect())
    }

    fn subsystems(&self, config: &tracker::Fields) -> Result<Vec<String>> {
        let field = &config.subsystems[0];

        let pool = self.fields.extra.get(field).ok_or_else(|| {
            eyre!(
                "The `{}` field is missing or has an unexpected structure in issue {}.",
                field,
                self.key
            )
        })?;

        let ssts: Vec<JiraSST> = serde_json::from_value(pool.clone()).wrap_err(eyre!(
            "The subsystems field has an unexpected structure in issue {}:\n{:#?}",
            self.key,
            pool
        ))?;

        let sst_names = ssts.into_iter().map(|sst| sst.value).collect();

        Ok(sst_names)
    }

    fn doc_text_status(&self, config: &tracker::Fields) -> Result<DocTextStatus> {
        let field = &config.doc_text_status[0];
        let rdt_field = self
            .fields
            .extra
            .get(field)
            .and_then(|rdt| rdt.get("value"))
            .and_then(Value::as_str)
            .ok_or_else(|| {
                eyre!(
                    "The `{}` field is missing or has an unexpected structure in issue {}.",
                    field,
                    self.key
                )
            })?;

        DocTextStatus::try_from(rdt_field)
    }

    fn docs_contact(&self, config: &tracker::Fields) -> DocsContact {
        let field = &config.docs_contact[0];
        let contact = self
            .fields
            .extra
            .get(field)
            .and_then(|cf| cf.get("emailAddress"))
            .and_then(Value::as_str)
            .map(ToString::to_string);

        if contact.is_none() {
            log::warn!(
                "The `{}` field is missing or has an unexpected structure in issue {}.",
                field,
                self.key
            );
        }

        DocsContact(contact)
    }

    fn url(&self, tracker: &tracker::Instance) -> String {
        format!("{}/browse/{}", tracker.host, &self.key)
    }
}
