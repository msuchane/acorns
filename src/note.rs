use crate::ticket_abstraction::AbstractTicket;

impl AbstractTicket {
    pub fn release_note(self) -> String {
        let empty = format!(
            ".🚧 {} | {}\n\n**No release note.** link:{}[]",
            self.summary, self.docs_contact, self.url
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
