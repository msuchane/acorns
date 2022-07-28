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
#[derive(Clone, Debug, PartialEq)]
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
            "?" | "Proposed" | "In progress" => Ok(Self::InProgress),
            // TODO: Does "Upstream only" really mean to skip this RN?
            "-" | "Rejected" | "Upstream only" => Ok(Self::NoDocumentation),
            _ => bail!("Unrecognized doc text status value: {:?}", string),
        }
    }
}

impl fmt::Display for DocTextStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let display = match self {
            Self::Approved => "RDT+",
            Self::InProgress => "RDT?",
            Self::NoDocumentation => "RDT-",
        };
        write!(f, "{}", display)
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
    fn docs_contact(&self, config: &tracker::Fields) -> Result<String>;
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
fn extract_field(extra: &Value, field: &str) -> Result<String> {
    extra
        .get(field)
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
        let field = &config.doc_type;
        extract_field(&self.extra, field)
            .wrap_err_with(|| eyre!("Failed to extract the doc type of bug {}.", self.id))
    }

    fn doc_text(&self, config: &tracker::Fields) -> Result<String> {
        let field = &config.doc_text;
        extract_field(&self.extra, field)
            .wrap_err_with(|| eyre!("Failed to extract the doc text of bug {}.", self.id))
    }

    fn target_releases(&self, config: &tracker::Fields) -> Result<Vec<String>> {
        let field = &config.target_release;
        let release = extract_field(&self.extra, field)
            .wrap_err_with(|| eyre!("Failed to extract the target release of bug {}.", self.id))?;

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
        let field = &config.subsystems;
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
        let flag = &config.doc_text_status;
        let rdt = self
            .get_flag(flag)
            // TODO: Make sure it's okay to quit with an error if RDT is missing.
            .ok_or_else(|| eyre!("The `{}` flag is missing.", flag));

        rdt.map(DocTextStatus::try_from)
            .wrap_err_with(|| eyre!("Failed to extract the doc text status of bug {}.", self.id))?
    }

    fn docs_contact(&self, _config: &tracker::Fields) -> Result<String> {
        // TODO: There's probably a way to avoid this clone.
        // Besides, this function exists only to satisfy the trait. It's very short and simple.
        Ok(self.docs_contact.clone())
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
        let field = &config.doc_type;
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
        let field = &config.doc_text;
        extract_field(&self.fields.extra, field)
            .wrap_err_with(|| eyre!("Failed to extract the doc text of issue {}.", self.id))
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
        let field = &config.subsystems;

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
        let field = &config.doc_text_status;
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

    fn docs_contact(&self, config: &tracker::Fields) -> Result<String> {
        let field = &config.docs_contact;
        self.fields
            .extra
            .get(field)
            .and_then(|cf| cf.get("emailAddress"))
            .and_then(Value::as_str)
            .map(ToString::to_string)
            .ok_or_else(|| {
                eyre!(
                    "The `{}` field is missing or has an unexpected structure in issue {}.",
                    field,
                    self.key
                )
            })
    }

    fn url(&self, tracker: &tracker::Instance) -> String {
        format!("{}/browse/{}", tracker.host, &self.id)
    }
}
