use std::fmt;

use crate::config::tracker::Service;
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

impl AbstractTicket {
    /// Compose a release note from an abstract ticket.
    pub fn release_note(&self) -> String {
        let docs_contact_placeholder = "No docs contact".to_string();
        let empty = format!(
            ".🚧 {} | {} | {}\n\n**No release note.** link:{}[]",
            self.summary,
            self.docs_contact
                .as_ref()
                .map_or(&docs_contact_placeholder, |dc| if dc.trim() == "" {
                    &docs_contact_placeholder
                } else {
                    dc
                }),
            self.requires_doc_text,
            self.url
        );
        if let Some(ref doc_text) = self.doc_text {
            if doc_text.trim() == "" {
                empty
            } else {
                // If the doc text contains DOS line endings (`\r`), remove them
                // and keep just UNIX endings (`\n`).
                let doc_text_unix = doc_text.replace('\r', "");
                format!("{}\n\n({})", doc_text_unix, self.format_signature())
            }
        } else {
            empty
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
}
