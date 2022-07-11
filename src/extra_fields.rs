use std::fmt;
use std::string::ToString;

use color_eyre::{
    eyre::{eyre, Context},
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

impl From<&str> for DocTextStatus {
    fn from(string: &str) -> Self {
        match string {
            "+" => Self::Approved,
            "?" => Self::InProgress,
            _ => Self::NoDocumentation,
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
    fn target_release(&self, config: &tracker::Fields) -> Result<String>;
    /// Extract the subsystems from the ticket.
    fn subsystems(&self, config: &tracker::Fields) -> Result<Vec<String>>;
    /// Extract the doc text status ("requires doc text") from the ticket.
    fn doc_text_status(&self, config: &tracker::Fields) -> DocTextStatus;
    /// Extract the docs contact from the ticket.
    fn docs_contact(&self, config: &tracker::Fields) -> Result<String>;
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
                "Field {} is missing or has an unexpected structure:\n{:#?}",
                field,
                extra.get(field)
            )
        })
}

impl ExtraFields for Bug {
    fn doc_type(&self, config: &tracker::Fields) -> Result<String> {
        let field = &config.doc_type;
        extract_field(&self.extra, field)
    }

    fn doc_text(&self, config: &tracker::Fields) -> Result<String> {
        let field = &config.doc_text;
        extract_field(&self.extra, field)
    }

    fn target_release(&self, config: &tracker::Fields) -> Result<String> {
        let field = &config.target_release;
        extract_field(&self.extra, field)
    }

    fn subsystems(&self, config: &tracker::Fields) -> Result<Vec<String>> {
        let field = &config.subsystems;
        let pool_field = self
            .extra
            .get(field)
            .ok_or_else(|| eyre!("Field {} is missing.", field))?;
        let pool: BzPool = serde_json::from_value(pool_field.clone())
            .context("Pool field has an unexpected structure.")?;

        // In Bugzilla, the bug always has just one subsystem. Therefore,
        // this returns a vector with a single item, or an empty vector.
        Ok(vec![pool.team.name])
    }

    fn doc_text_status(&self, config: &tracker::Fields) -> DocTextStatus {
        let flag = &config.doc_text_status;
        let rdt = self.get_flag(flag);

        if let Some(rdt) = rdt {
            DocTextStatus::from(rdt)
        } else {
            // If the RDT flag is completely missing, use `-` as the default.
            log::warn!("Bug {} is missing the `requires_doc_text` flag.", self.id);
            DocTextStatus::NoDocumentation
        }
    }

    fn docs_contact(&self, _config: &tracker::Fields) -> Result<String> {
        // TODO: There's probably a way to avoid this clone.
        // Besides, this function exists only to satisfy the trait. It's very short and simple.
        Ok(self.docs_contact.clone())
    }
}

#[derive(Deserialize, Debug)]
struct JiraDocType {
    value: String,
}

impl ExtraFields for Issue {
    fn doc_type(&self, config: &tracker::Fields) -> Result<String> {
        let field = &config.doc_type;
        let doc_type_field = self
            .fields
            .extra
            .get(field)
            .ok_or_else(|| eyre!("Field {} is missing.", field))?;
        let doc_type: JiraDocType = serde_json::from_value(doc_type_field.clone())
            .context("Jira doc type field has an unexpected structure.")?;

        Ok(doc_type.value)
    }

    fn doc_text(&self, config: &tracker::Fields) -> Result<String> {
        let field = &config.doc_text;
        self.fields
            .extra
            .get(field)
            .and_then(Value::as_str)
            .map(ToString::to_string)
            .ok_or_else(|| eyre!("Field {} is missing or has an unexpected structure.", field))
    }

    fn target_release(&self, _config: &tracker::Fields) -> Result<String> {
        self.fields
            .fix_versions
            // TODO: Is the first fix version in the list the one that we want?
            .get(0)
            // This error is not serious. Recover from it in the higher layers.
            .ok_or_else(|| eyre!("Issue {} has no fix version.", &self.key))
            // TODO: Get rid of the clone.
            .map(|version| version.name.clone())
    }

    fn subsystems(&self, config: &tracker::Fields) -> Result<Vec<String>> {
        let field = &config.subsystems;

        Ok(self
            .fields
            .extra
            .get(field)
            .and_then(Value::as_array)
            .unwrap()
            .iter()
            // TODO: Handle the errors more safely, without unwraps.
            .map(|sst| sst.get("value").unwrap().as_str().unwrap().to_string())
            .collect())
    }

    fn doc_text_status(&self, config: &tracker::Fields) -> DocTextStatus {
        let field = &config.doc_text_status;
        let rdt_field = self.fields.extra.get(field);

        rdt_field
            .and_then(|rdt| rdt.get("value"))
            .and_then(Value::as_str)
            .map_or(DocTextStatus::NoDocumentation, DocTextStatus::from)
    }

    fn docs_contact(&self, config: &tracker::Fields) -> Result<String> {
        let field = &config.docs_contact;
        self.fields
            .extra
            .get(field)
            .and_then(|cf| cf.get("emailAddress"))
            .and_then(Value::as_str)
            .map(ToString::to_string)
            .ok_or_else(|| eyre!("Field {} is missing or has an unexpected structure.", field))
    }
}
