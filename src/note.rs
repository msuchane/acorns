use std::fmt;

use crate::config::tracker::Service;
use crate::templating::DocumentVariant;
use crate::ticket_abstraction::AbstractTicket;

impl fmt::Display for Service {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let name = match self {
            Self::Bugzilla => "Bugzilla",
            Self::Jira => "Jira",
        };
        write!(f, "{}", name)
    }
}

impl<'a> AbstractTicket<'a> {
    /// Compose a release note from an abstract ticket.
    pub fn release_note(&self, variant: &DocumentVariant) -> String {
        let docs_contact_placeholder = "No docs contact";

        // TODO: Handle the empty docs contact earlier as an error.
        let docs_contact = if self.docs_contact.is_empty() {
            docs_contact_placeholder
        } else {
            &self.docs_contact
        };

        // This debug information line appears at empty release notes
        // and everywhere in the Internal document variant.
        let debug_info = format!(
            "| {} | {} | link:{}[]",
            docs_contact, self.doc_text_status, self.url
        );

        // A placeholder for release notes with an empty doc text.
        let empty = format!(
            ".🚧 {} {} \n\n**No release note.**",
            self.summary, debug_info,
        );

        // TODO: Handle the empty doc text earlier as an error.
        if self.doc_text.is_empty() {
            empty
        } else {
            // If the doc text contains DOS line endings (`\r`), remove them
            // and keep just UNIX endings (`\n`).
            let doc_text_unix = self.doc_text.replace('\r', "");

            // This is the resulting release note:
            format!(
                "{}\n\n({}) {}",
                doc_text_unix,
                self.format_signature(),
                // In the internal variant, add the debug information line.
                if *variant == DocumentVariant::Internal {
                    debug_info
                } else {
                    String::new()
                },
            )
        }
    }

    /// Prepare the link or the non-clickable signature that marks the ticket
    /// belonging to this release note.
    fn format_signature(&self) -> String {
        let label = format!("{}:{}", self.id.tracker, self.id.key);
        if self.public {
            format!("link:{}[{}]", self.url, label)
        } else {
            label
        }
    }

    /// Construct a URL back to the original ticket online.
    pub fn url(&self) -> String {
        todo!()
    }
}
