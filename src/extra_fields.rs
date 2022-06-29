use bugzilla_query::Bug;
use jira_query::Issue;

pub trait ExtraFields {
    /// Extract the doc type from the ticket.
    fn doc_type(&self) -> Option<String>;
    /// Extract the doc text from the ticket.
    fn doc_text(&self) -> Option<String>;
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
}
