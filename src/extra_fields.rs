use std::fmt;
use std::string::ToString;

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
    fn doc_type(&self, config: &tracker::Fields) -> Option<String>;
    /// Extract the doc text from the ticket.
    fn doc_text(&self, config: &tracker::Fields) -> Option<String>;
    /// Extract the target release from the ticket.
    fn target_release(&self, config: &tracker::Fields) -> Option<String>;
    /// Extract the subsystems from the ticket.
    fn subsystems(&self, config: &tracker::Fields) -> Vec<String>;
    /// Extract the doc text status ("requires doc text") from the ticket.
    fn doc_text_status(&self, config: &tracker::Fields) -> DocTextStatus;
}

#[derive(Deserialize, Debug)]
struct BzPool {
    team: BzTeam,
}

#[derive(Deserialize, Debug)]
struct BzTeam {
    name: String,
}

impl ExtraFields for Bug {
    // TODO: The following two fields should be configurable by tracker.
    // Also, handle the errors properly. For now, we're just assuming that the fields
    // are strings, and panicking if not.
    fn doc_type(&self, config: &tracker::Fields) -> Option<String> {
        self.extra
            .get("cf_doc_type")
            .and_then(Value::as_str)
            .map(ToString::to_string)
    }

    fn doc_text(&self, config: &tracker::Fields) -> Option<String> {
        self.extra
            .get("cf_release_notes")
            .and_then(Value::as_str)
            .map(ToString::to_string)
    }

    fn target_release(&self, config: &tracker::Fields) -> Option<String> {
        self.extra
            .get("cf_internal_target_release")
            .and_then(Value::as_str)
            .map(ToString::to_string)
    }

    fn subsystems(&self, config: &tracker::Fields) -> Vec<String> {
        let pool_field = self.extra.get("pool").expect("Bug has no pool field.");
        let pool: BzPool = serde_json::from_value(pool_field.clone())
            .expect("Pool field has an unexpected structure.");

        // In Bugzilla, the bug always has just one subsystem. Therefore,
        // this returns a vector with a single item, or an empty vector.
        vec![pool.team.name]
    }

    fn doc_text_status(&self, config: &tracker::Fields) -> DocTextStatus {
        let rdt = self.get_flag("requires_doc_text");

        if let Some(rdt) = rdt {
            DocTextStatus::from(rdt)
        } else {
            // If the RDT flag is completely missing, use `-` as the default.
            log::warn!("Bug {} is missing the `requires_doc_text` flag.", self.id);
            DocTextStatus::NoDocumentation
        }
    }
}

#[derive(Deserialize, Debug)]
struct JiraDocType {
    value: String,
}

impl ExtraFields for Issue {
    // TODO: The following two fields should be configurable by tracker.
    fn doc_type(&self, config: &tracker::Fields) -> Option<String> {
        let doc_type_field = self.fields.extra.get("customfield_12317310")?;
        let doc_type: JiraDocType = serde_json::from_value(doc_type_field.clone())
            // TODO: Consolidate the previous Option and this Result properly.
            .expect("Doc type field has an unexpected structure.");

        Some(doc_type.value)
    }

    fn doc_text(&self, config: &tracker::Fields) -> Option<String> {
        self.fields
            .extra
            .get("customfield_12317322")
            .map(|value| value.as_str().unwrap().to_string())
    }

    fn target_release(&self, config: &tracker::Fields) -> Option<String> {
        self.fields
            .fix_versions
            // TODO: Is the first fix version in the list the one that we want?
            .get(0)
            // TODO: Get rid of the clone.
            .map(|version| version.name.clone())
    }

    fn subsystems(&self, config: &tracker::Fields) -> Vec<String> {
        self.fields
            .extra
            // This is the "Pool Team" field.
            .get("customfield_12317259")
            .and_then(Value::as_array)
            .unwrap()
            .iter()
            // TODO: Handle the errors more safely, without unwraps.
            .map(|sst| sst.get("value").unwrap().as_str().unwrap().to_string())
            .collect()
    }

    fn doc_text_status(&self, config: &tracker::Fields) -> DocTextStatus {
        let rdt_field = self
            .fields
            .extra
            // TODO: This field should be configurable.
            .get("customfield_12317337");

        rdt_field
            .and_then(|rdt| rdt.get("value"))
            .and_then(Value::as_str)
            .map_or(DocTextStatus::NoDocumentation, DocTextStatus::from)
    }
}
