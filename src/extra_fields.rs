use std::fmt;

use bugzilla_query::Bug;
use jira_query::Issue;

/// The status or progress of the release note.
#[derive(Clone, Debug)]
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
    fn doc_type(&self) -> Option<String>;
    /// Extract the doc text from the ticket.
    fn doc_text(&self) -> Option<String>;
    /// Extract the target release from the ticket.
    fn target_release(&self) -> Option<String>;
    /// Extract the subsystems from the ticket.
    fn subsystems(&self) -> Vec<String>;
    /// Extract the doc text status ("requires doc text") from the ticket.
    fn doc_text_status(&self) -> DocTextStatus;
}

impl ExtraFields for Bug {
    // TODO: The following two fields should be configurable by tracker.
    // Also, handle the errors properly. For now, we're just assuming that the fields
    // are strings, and panicking if not.
    fn doc_type(&self) -> Option<String> {
        self.extra
            .get("cf_doc_type")
            .map(|dt| dt.as_str().unwrap().to_string())
    }

    fn doc_text(&self) -> Option<String> {
        self.extra
            .get("cf_release_notes")
            .map(|rn| rn.as_str().unwrap().to_string())
    }

    fn target_release(&self) -> Option<String> {
        self.extra
            .get("cf_internal_target_release")
            .map(|itr| itr.as_str().unwrap().to_string())
    }

    fn subsystems(&self) -> Vec<String> {
        self.extra
            .get("pool")
            .and_then(|pool| pool.get("team"))
            .and_then(|team| team.get("name"))
            // In Bugzilla, the bug always has just one subsystem. Therefore,
            // this returns a vector with a single item, or an empty vector.
            .map_or_else(Vec::new, |name| vec![name.as_str().unwrap().to_string()])
    }

    fn doc_text_status(&self) -> DocTextStatus {
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

impl ExtraFields for Issue {
    // TODO: The following two fields should be configurable by tracker.
    // Also, handle the errors properly.
    fn doc_type(&self) -> Option<String> {
        self.fields
            .extra
            .get("customfield_12317310")
            // This chain of `and_then` and `map` handles the two consecutive Options:
            // The result is a String only when neither Option is None.
            // The first method is `and_then` rather than `map` to avoid a nested Option.
            .and_then(|cf| cf.get("value"))
            .map(|v| v.as_str().unwrap().to_string())
    }

    fn doc_text(&self) -> Option<String> {
        self.fields
            .extra
            .get("customfield_12317322")
            .map(|value| value.as_str().unwrap().to_string())
    }

    fn target_release(&self) -> Option<String> {
        self.fields
            .fix_versions
            // TODO: Is the first fix version in the list the one that we want?
            .get(0)
            // TODO: Get rid of the clone.
            .map(|version| version.name.clone())
    }

    fn subsystems(&self) -> Vec<String> {
        self.fields
            .extra
            // This is the "Pool Team" field.
            .get("customfield_12317259")
            .and_then(|ssts| ssts.as_array())
            .unwrap()
            .iter()
            // TODO: Handle the errors more safely, without unwraps.
            .map(|sst| sst.get("value").unwrap().as_str().unwrap().to_string())
            .collect()
    }

    fn doc_text_status(&self) -> DocTextStatus {
        let rdt_field = self
            .fields
            .extra
            // TODO: This field should be configurable.
            .get("customfield_12317337");

        rdt_field
            .and_then(|rdt| rdt.get("value"))
            .and_then(|rdt_value| rdt_value.as_str())
            .map_or(DocTextStatus::NoDocumentation, DocTextStatus::from)
    }
}
