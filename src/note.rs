use crate::ticket_abstraction::AbstractTicket;

impl AbstractTicket {
    pub fn release_note(self) -> String {
        self.doc_text.unwrap_or("No release note.".to_string())
    }
}
