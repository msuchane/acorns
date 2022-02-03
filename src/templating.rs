use std::fs;
use std::path::Path;

use color_eyre::eyre::{Context, Result};
use log::debug;
use serde::Deserialize;

use crate::ticket_abstraction::AbstractTicket;

#[derive(Debug, PartialEq, Deserialize)]
pub struct Template {
    pub chapters: Vec<Section>,
    pub sections: Option<Vec<Section>>,
}

#[derive(Debug, PartialEq, Deserialize)]
pub struct Section {
    pub title: String,
    pub filter: Filter,
    pub sections: Option<Vec<Section>>,
}

#[derive(Debug, PartialEq, Deserialize)]
pub struct Filter {
    pub doc_type: Option<Vec<String>>,
    pub subsystem: Option<Vec<String>>,
    pub component: Option<Vec<String>>,
}

impl Section {
    fn render(&self, tickets: &[AbstractTicket]) -> String {
        let heading = format!("= {}", self.title);
        let matching_tickets = tickets.iter().filter(|t| {
            self.filter
                .doc_type
                .as_ref()
                .unwrap()
                .contains(t.doc_type.as_ref().unwrap())
        });
        let release_notes: Vec<_> = matching_tickets.map(|t| t.release_note()).collect();
        format!("{}\n\n{}", heading, release_notes.join("\n\n"))
    }
}

pub fn parse(template_file: &Path) -> Result<Template> {
    let text = fs::read_to_string(template_file).context("Cannot read the template file.")?;
    let templates: Template =
        serde_yaml::from_str(&text).context("Cannot parse the template file.")?;
    debug!("{:#?}", templates);
    Ok(templates)
}

pub fn format_document(tickets: &[AbstractTicket], template: Template) -> String {
    let release_notes: Vec<String> = tickets.into_iter().map(|t| t.release_note()).collect();
    // let document = format!("= Release notes\n\n{}", release_notes.join("\n\n"));
    // debug!("Release notes:\n\n{}", document);

    let chapters: Vec<_> = template
        .chapters
        .iter()
        .map(|t| t.render(tickets))
        .collect();
    debug!("Chapters: {:#?}", chapters);

    let document = chapters.join("\n\n");
    debug!("Document: {}", document);
    document
}
