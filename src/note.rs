/*
cizrna: Generate an AsciiDoc release notes document from tracking tickets.
Copyright (C) 2022  Marek Suchánek  <msuchane@redhat.com>

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

use crate::templating::DocumentVariant;
use crate::ticket_abstraction::AbstractTicket;

impl AbstractTicket {
    /// Compose a release note from an abstract ticket.
    pub fn release_note(&self, variant: DocumentVariant) -> String {
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
            docs_contact, self.doc_text_status, &self.url
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
                "{}\n\n{} {}",
                doc_text_unix,
                self.all_signatures(),
                // In the internal variant, add the debug information line.
                if variant == DocumentVariant::Internal {
                    debug_info
                } else {
                    String::new()
                },
            )
        }
    }

    /// Prepare the link or the non-clickable signature that marks the ticket
    /// belonging to this release note.
    pub fn signature(&self) -> String {
        if self.public {
            format!("link:{}[{}]", &self.url, self.id)
        } else {
            self.id.to_string()
        }
    }

    /// Prepare a list with signatures to this ticket and all its optional references.
    /// The result is a comma-separated list of signatures, enclosed in parentheses.
    fn all_signatures(&self) -> String {
        if let Some(references) = &self.references {
            format!("({}, {})", self.signature(), references.join(", "))
        } else {
            format!("({})", self.signature())
        }
    }
}
