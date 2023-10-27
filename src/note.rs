/*
acorns: Generate an AsciiDoc release notes document from tracking tickets.
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
    #[must_use]
    pub fn release_note(&self, variant: DocumentVariant, with_priv_footnote: bool) -> String {
        let anchor = self.anchor_declaration();

        // This debug information line appears at empty release notes
        // and everywhere in the Internal document variant.
        let debug_info = format!(
            "| {} | {} | link:{}[]",
            &self.docs_contact, self.doc_text_status, &self.url
        );

        // A placeholder for release notes with an empty doc text.
        let empty = format!(
            "{}\n.🚧 {} {} \n\n**No release note.**",
            anchor, self.summary, debug_info,
        );

        // TODO: Handle the empty doc text earlier as an error.
        if content_lines(&self.doc_text).is_empty() {
            empty
        } else {
            // If the doc text contains DOS line endings (`\r`), remove them
            // and keep just UNIX endings (`\n`).
            let doc_text_unix = self.doc_text.replace('\r', "");

            // This is the resulting release note:
            format!(
                "{}\n{}\n\n{} {}",
                anchor,
                doc_text_unix,
                self.all_signatures(with_priv_footnote),
                // In the internal variant, add the debug information line.
                if variant == DocumentVariant::Internal {
                    &debug_info
                } else {
                    ""
                },
            )
        }
    }

    /// Prepare the link or the non-clickable signature that marks the ticket
    /// belonging to this release note.
    ///
    /// For example, `link:https://...bugzilla...12345[BZ#12345]`.
    #[must_use]
    pub fn signature(&self, with_priv_footnote: bool) -> String {
        let id = &self.id;

        if self.public {
            // If the ticket is public, add a clickable link.
            format!("link:{}[{}]", &self.url, id)
        } else {
            // If the ticket is private, and the project configures a dedicated footnote,
            // add a footnote that explains why the link isn't clickable.
            // This uses the deprecated AsciiDoc `footnoteref` syntax
            // so that you can build the document with very outdated asciidoctor.
            if with_priv_footnote {
                format!("{id}footnoteref:[PrivateTicketFootnote]")
            } else {
                id.to_string()
            }
        }
    }

    /// Prepare a list with signatures to this ticket and all its optional references.
    /// The result is a comma-separated list of signatures, enclosed in parentheses.
    #[must_use]
    fn all_signatures(&self, with_priv_footnote: bool) -> String {
        let mut signatures = vec![self.signature(with_priv_footnote)];

        if let Some(references) = self.references.as_ref() {
            signatures.append(&mut references.clone());
        }

        signatures.join(", ")
    }

    /// Format an ID, or an anchor, that this release note can set and that you can use
    /// to refer back to this release note from elsewhere.
    ///
    /// For example, `BZ-12345`.
    #[must_use]
    pub fn anchor(&self) -> String {
        let service = self.id.tracker.short_name();
        let key = &self.id.key;

        // TODO: This anchor isn't unique across the document if the RN is reused.
        format!("{service}-{key}")
    }

    /// Format an AsciiDoc ID line that sets an HTML anchor.
    ///
    /// For example, `[id="BZ-12345"]`.
    fn anchor_declaration(&self) -> String {
        let anchor = self.anchor();

        format!("[id=\"{anchor}\"]")
    }

    /// Format a reference using the xref syntax that points back to this release note.
    #[must_use]
    pub fn xref(&self) -> String {
        let anchor = self.anchor();
        let id = self.id.to_string();

        format!("xref:{anchor}[{id}]")
    }
}

/// Pull out the lines from a doc text that aren't empty and aren't comments.
/// In other words, this should be the actual text content of the release note.
pub fn content_lines(doc_text: &str) -> Vec<&str> {
    doc_text
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with("//"))
        .collect()
}
