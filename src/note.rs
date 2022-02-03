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
    pub fn release_note(&self) -> String {
        let docs_contact_placeholder = "No docs contact".to_string();
        let empty = format!(
            ".🚧 {} | {} | {}\n\n**No release note.** link:{}[]",
            self.summary,
            self.docs_contact
                .as_ref()
                .unwrap_or(&docs_contact_placeholder),
            self.requires_doc_text,
            self.url
        );
        if let Some(ref doc_text) = self.doc_text {
            if doc_text.trim() == "" {
                empty
            } else {
                format!("{}\n\n({})", doc_text, self.format_signature())
            }
        } else {
            empty
        }
    }

    fn format_signature(&self) -> String {
        let label = format!("{}:{}", self.id.tracker, self.id.key);
        if self.public {
            format!("link:{}[{}]", self.url, label)
        } else {
            label
        }
    }
}
