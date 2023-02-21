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
    eyre::{bail, eyre},
    Report, Result,
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

/// All the extra fields, so that we can implement a standardized
/// user display string on them.
#[derive(Clone, Copy)]
enum Field {
    DocType,
    DocText,
    TargetRelease,
    Subsystems,
    DocTextStatus,
    DocsContact,
}

impl fmt::Display for Field {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::DocType => write!(f, "doc type"),
            Self::DocText => write!(f, "doc text"),
            Self::TargetRelease => write!(f, "target release"),
            Self::Subsystems => write!(f, "subsystems"),
            Self::DocTextStatus => write!(f, "doc text status"),
            Self::DocsContact => write!(f, "docs contact"),
        }
    }
}

pub trait ExtraFields {
    /// Extract the doc type from the ticket.
    fn doc_type(&self, config: &impl tracker::FieldsConfig) -> Result<String>;
    /// Extract the doc text from the ticket.
    fn doc_text(&self, config: &impl tracker::FieldsConfig) -> Result<String>;
    /// Extract the target release from the ticket.
    fn target_releases(&self, config: &impl tracker::FieldsConfig) -> Vec<String>;
    /// Extract the subsystems from the ticket.
    fn subsystems(&self, config: &impl tracker::FieldsConfig) -> Result<Vec<String>>;
    /// Extract the doc text status ("requires doc text") from the ticket.
    fn doc_text_status(&self, config: &impl tracker::FieldsConfig) -> Result<DocTextStatus>;
    /// Extract the docs contact from the ticket.
    fn docs_contact(&self, config: &impl tracker::FieldsConfig) -> DocsContact;
    /// Construct a URL back to the original ticket online.
    fn url(&self, tracker: &impl tracker::FieldsConfig) -> String;
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
fn extract_field(field_name: Field, extra: &Value, fields: &[String], id: Id) -> Result<String> {
    // Record all errors that occur with tried fields that exist.
    let mut errors = Vec::new();
    // Record all empty but potentially okay fields.
    let mut empty_fields: Vec<&str> = Vec::new();

    for field in fields {
        let field_value = extra.get(field);

        // See if the field even exists in the first place.
        if let Some(value) = field_value {
            // This check covers the case where the field exists, but its value
            // is unset. I think it's safe to treat it as an empty string.
            if let Value::Null = value {
                empty_fields.push(field);
            }

            // The field exists and has a Some value. Try converting it to a string.
            let try_string = value.as_str().map(ToString::to_string);

            if let Some(string) = try_string {
                return Ok(string);
            } else {
                let error = eyre!("Field `{field}` is not a string: {value:?}");
                errors.push(error);
            }
        } else {
            // The field doesn't exist.
            let error = eyre!("Field `{field}` is missing.");
            errors.push(error);
        }
    }

    // If all we've got are errors, return an error with the complete errors report.
    if empty_fields.is_empty() {
        let report = error_chain(errors, field_name, fields, id);
        Err(report)
    // If we at least got an existing but empty field, return an empty string.
    // I think it's safe to treat it as such.
    } else {
        log::warn!("Fields are empty in {}: {:?}", id, empty_fields);
        Ok(String::new())
    }
}

/// An enum to standardize the error reporting of Bugzilla and Jira tickets.
#[derive(Clone, Copy)]
enum Id<'a> {
    BZ(i32),
    Jira(&'a str),
}

impl fmt::Display for Id<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::BZ(id) => write!(f, "bug {id}"),
            Self::Jira(id) => write!(f, "ticket {id}"),
        }
    }
}

/// Prepare a user-readable list of errors, reported in the order that they occurred.
fn error_chain(mut errors: Vec<Report>, field_name: Field, fields: &[String], id: Id) -> Report {
    let top_error = eyre!(
        "The {} field is missing or malformed in {}.\n\
        The configured fields are: {:?}",
        field_name,
        id,
        fields
    );

    errors.reverse();

    let report = errors.into_iter().reduce(Report::wrap_err);

    match report {
        Some(report) => report.wrap_err(top_error),
        None => top_error,
    }
}

impl ExtraFields for Bug {
    fn doc_type(&self, config: &impl tracker::FieldsConfig) -> Result<String> {
        let fields = config.doc_type();
        extract_field(Field::DocType, &self.extra, fields, Id::BZ(self.id))
    }

    fn doc_text(&self, config: &impl tracker::FieldsConfig) -> Result<String> {
        let fields = config.doc_text();
        extract_field(Field::DocText, &self.extra, fields, Id::BZ(self.id))
    }

    fn target_releases(&self, config: &impl tracker::FieldsConfig) -> Vec<String> {
        let fields = config.target_release();
        let mut errors = Vec::new();

        // Try the custom overrides, if any.
        match extract_field(Field::TargetRelease, &self.extra, fields, Id::BZ(self.id)) {
            Ok(release) => {
                // Bugzilla uses the "---" placeholder to represent an unset release.
                // TODO: Are there any more placeholder?
                let empty_values = ["---"];

                // If the release is unset, return no releases. If it's set, return that one release.
                let in_list = if empty_values.contains(&release.as_str()) {
                    vec![]
                } else {
                    vec![release]
                };
                return in_list;
            }
            Err(error) => {
                // The target release field isn't critical. Record the problem
                // and proceed.
                errors.push(error);
            }
        }

        // Fall back on the standard field
        match &self.target_release {
            Some(bugzilla_query::Version::One(version)) => vec![version.clone()],
            Some(bugzilla_query::Version::Many(versions)) => versions.clone(),
            None => {
                let report = error_chain(errors, Field::TargetRelease, fields, Id::BZ(self.id));
                log::warn!("{report}");

                // Finally, return an empty list if everything else failed.
                Vec::new()
            }
        }
    }

    fn subsystems(&self, config: &impl tracker::FieldsConfig) -> Result<Vec<String>> {
        let fields = config.subsystems();
        let mut errors = Vec::new();

        for field in fields {
            let pool_field = self.extra.get(field);

            if let Some(pool_field) = pool_field {
                let pool: Result<BzPool, serde_json::Error> =
                    serde_json::from_value(pool_field.clone());

                match pool {
                    // In Bugzilla, the bug always has just one subsystem. Therefore,
                    // this returns a vector with a single item, or an empty vector.
                    Ok(pool) => {
                        return Ok(vec![pool.team.name]);
                    }

                    // If the parsing resulted in an error, save the error for later.
                    Err(error) => errors.push(error.into()),
                }
            } else {
                let error = eyre!("Field `{}` is missing", field);
                errors.push(error);
            }
        }

        let report = error_chain(errors, Field::Subsystems, fields, Id::BZ(self.id));
        Err(report)
    }

    /// If the flag is unset, treat it only as a warning, not a breaking error,
    /// and proceed with the default value.
    /// An unset RDT is a relatively common occurrence on Bugzilla.
    fn doc_text_status(&self, config: &impl tracker::FieldsConfig) -> Result<DocTextStatus> {
        let fields = config.doc_text_status();
        let mut errors = Vec::new();
        // Record all empty but potentially okay fields.
        let mut empty_fields: Vec<&str> = Vec::new();

        // If the RDT flag is unset, use this:
        let default_rdt = DocTextStatus::InProgress;

        for flag in fields {
            if let Some(rdt) = self.get_flag(flag) {
                match DocTextStatus::try_from(rdt) {
                    Ok(status) => {
                        return Ok(status);
                    }
                    Err(error) => {
                        errors.push(eyre!(
                            "Failed to extract the doc text status from flag {}.",
                            flag
                        ));
                        errors.push(error);
                    }
                }
            } else {
                empty_fields.push(flag);
            }
        }

        // If all we've got are errors, return an error with the complete errors report.
        if empty_fields.is_empty() {
            let report = error_chain(errors, Field::DocTextStatus, fields, Id::BZ(self.id));
            Err(report)
        // If we at least got an existing but empty field, return the default value.
        } else {
            log::warn!(
                "Flags are empty in {}: {}",
                Id::BZ(self.id),
                empty_fields.join(", ")
            );
            Ok(default_rdt)
        }
    }

    fn docs_contact(&self, config: &impl tracker::FieldsConfig) -> DocsContact {
        let fields = config.docs_contact();
        let mut errors = Vec::new();

        // Try the custom overrides, if any.
        let docs_contact = extract_field(Field::DocsContact, &self.extra, fields, Id::BZ(self.id));

        match docs_contact {
            Ok(docs_contact) => {
                return DocsContact(Some(docs_contact));
            }
            Err(error) => {
                errors.push(error);
            }
        }

        // No override succeeded. See if there's a value in the standard field.
        if self.docs_contact.is_none() {
            let report = error_chain(errors, Field::DocsContact, fields, Id::BZ(self.id));
            log::warn!("{:?}", report);
        }

        // TODO: There's probably a way to avoid this clone.
        DocsContact(self.docs_contact.clone())
    }

    fn url(&self, tracker: &impl tracker::FieldsConfig) -> String {
        format!("{}/show_bug.cgi?id={}", tracker.host(), self.id)
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
    fn doc_type(&self, config: &impl tracker::FieldsConfig) -> Result<String> {
        let fields = config.doc_type();
        let mut errors = Vec::new();

        for field in fields {
            let doc_type_field = self.fields.extra.get(field);

            if let Some(doc_type_field) = doc_type_field {
                let doc_type: Result<JiraDocType, serde_json::Error> =
                    serde_json::from_value(doc_type_field.clone());

                match doc_type {
                    Ok(doc_type) => {
                        return Ok(doc_type.value);
                    }
                    Err(error) => {
                        errors.push(eyre!(
                            "The `{}` field has an unexpected structure:\n{:#?}",
                            field,
                            doc_type_field
                        ));
                        errors.push(error.into());
                    }
                }
            } else {
                errors.push(eyre!("The `{field}` field is missing."));
            };
        }

        let report = error_chain(errors, Field::DocType, fields, Id::Jira(&self.key));
        Err(report)
    }

    fn doc_text(&self, config: &impl tracker::FieldsConfig) -> Result<String> {
        let fields = config.doc_text();
        extract_field(
            Field::DocText,
            &self.fields.extra,
            fields,
            Id::Jira(&self.key),
        )
    }

    fn target_releases(&self, config: &impl tracker::FieldsConfig) -> Vec<String> {
        let fields = config.target_release();
        let mut errors = Vec::new();

        for field in fields {
            if let Some(value) = self.fields.extra.get(field) {
                // Try to deserialize as the standard fix versions, only in a custom field.
                let jira_versions: Result<Vec<jira_query::Version>, serde_json::Error> =
                    serde_json::from_value(value.clone());
                match jira_versions {
                    Ok(vec) => {
                        let versions: Vec<String> =
                            vec.iter().map(|version| version.name.clone()).collect();
                        return versions;
                    }
                    Err(error) => {
                        errors.push(error.into());
                    }
                }

                // Try to deserialize as a simple list of strings.
                let string_versions: Result<Vec<String>, serde_json::Error> =
                    serde_json::from_value(value.clone());
                match string_versions {
                    Ok(vec) => {
                        return vec;
                    }
                    Err(error) => {
                        errors.push(error.into());
                    }
                }

                // Try to deserialize as a single string.
                let string = extract_field(
                    Field::TargetRelease,
                    &self.extra,
                    &[field.clone()],
                    Id::Jira(&self.key),
                );
                match string {
                    Ok(string) => {
                        return vec![string];
                    }
                    Err(error) => {
                        errors.push(error);
                    }
                }
            } else {
                errors.push(eyre!("The `{field}` field is missing"));
            }
        }

        // If any errors occurred, report them as warnings and continue.
        if !errors.is_empty() {
            let id = Id::Jira(&self.key);
            let report = error_chain(errors, Field::TargetRelease, fields, id);
            log::warn!("The custom target releases failed in {}. Falling back on the standard fix versions field.", id);

            // Provide this additional information on demand.
            log::debug!("{:?}", report);
        }

        // Always fall back on the standard field.
        let standard_field = self
            .fields
            .fix_versions
            .iter()
            // TODO: Get rid of the clone if possible
            .map(|version| version.name.clone())
            .collect();

        standard_field
    }

    fn subsystems(&self, config: &impl tracker::FieldsConfig) -> Result<Vec<String>> {
        let fields = config.subsystems();
        // Record all errors that occur with tried fields that exist.
        let mut errors = Vec::new();

        for field in fields {
            let pool = self.fields.extra.get(field);

            if let Some(pool) = pool {
                let ssts: Result<Vec<JiraSST>, serde_json::Error> =
                    serde_json::from_value(pool.clone());

                // If the field exist, try parsing it and returning the result.
                // If the parsing fails, record the error for later.
                match ssts {
                    Ok(ssts) => {
                        let sst_names = ssts.into_iter().map(|sst| sst.value).collect();
                        return Ok(sst_names);
                    }
                    Err(error) => {
                        errors.push(error.into());
                    }
                }
            }
        }

        // No field produced a `Some` value.
        // Prepare a user-readable list of errors, if any occurred.
        let report = error_chain(errors, Field::Subsystems, fields, Id::Jira(&self.key));

        // Return the combined error.
        Err(report)
    }

    fn doc_text_status(&self, config: &impl tracker::FieldsConfig) -> Result<DocTextStatus> {
        let fields = config.doc_text_status();
        for field in fields {
            let rdt_field = self
                .fields
                .extra
                .get(field)
                .and_then(|rdt| rdt.get("value"))
                .and_then(Value::as_str);

            if let Some(rdt_field) = rdt_field {
                return DocTextStatus::try_from(rdt_field);
            };
        }

        // No field produced a `Some` value.
        let report = error_chain(
            Vec::new(),
            Field::DocTextStatus,
            fields,
            Id::Jira(&self.key),
        );
        Err(report)
    }

    fn docs_contact(&self, config: &impl tracker::FieldsConfig) -> DocsContact {
        let fields = config.docs_contact();

        for field in fields {
            let contact = self
                .fields
                .extra
                .get(field)
                .and_then(|cf| cf.get("emailAddress"))
                .and_then(Value::as_str)
                .map(ToString::to_string);

            if contact.is_some() {
                return DocsContact(contact);
            }
        }

        // No field produced a `Some` value.
        let report = error_chain(Vec::new(), Field::DocsContact, fields, Id::Jira(&self.key));
        // This field is non-critical.
        log::warn!("{:?}", report);

        DocsContact(None)
    }

    fn url(&self, tracker: &impl tracker::FieldsConfig) -> String {
        format!("{}/browse/{}", tracker.host(), &self.key)
    }
}
