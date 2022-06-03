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

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Module {
    pub file_name: String,
    pub text: String,
    pub included_modules: Option<Vec<Self>>,
}

impl Module {
    pub fn include_statement(&self) -> String {
        format!("include::{}[leveloffset=+1]", &self.file_name)
    }
}

impl Section {
    fn render(&self, id: &str, tickets: &[AbstractTicket]) -> String {
        let heading = format!("= {}", &self.title);
        let matching_tickets = tickets.iter().filter(|&t| self.matches_ticket(t));
        let release_notes: Vec<_> = matching_tickets.map(|t| t.release_note()).collect();
        format!(
            "[id=\"{}\"]\n{}\n\n{}",
            id,
            heading,
            release_notes.join("\n\n")
        )
    }

    fn modules(&self, tickets: &[AbstractTicket], prefix: Option<&str>) -> Module {
        let matching_tickets: Vec<AbstractTicket> = tickets
            .iter()
            .filter(|&t| self.matches_ticket(t))
            .cloned()
            .collect();

        let module_id_fragment = self.title.to_lowercase().replace(' ', "-");
        let module_id = if let Some(prefix) = prefix {
            format!("{}-{}", prefix, module_id_fragment)
        } else {
            module_id_fragment
        };

        // If the section includes other sections, treat it as an assembly.
        if let Some(sections) = &self.sections {
            let file_name = format!("assembly_{}.adoc", module_id);
            let included_modules: Vec<Module> = sections
                .iter()
                .map(|s| s.modules(&matching_tickets, Some(&module_id)))
                .collect();
            let include_statements: Vec<String> = included_modules
                .iter()
                .map(|m| m.include_statement())
                .collect();
            let include_block = include_statements.join("\n\n");
            let text = format!(
                "[id=\"{}\"]\n= {}\n\n{}",
                &module_id, &self.title, include_block
            );

            Module {
                file_name,
                text,
                included_modules: Some(included_modules),
            }
        // If the section includes no sections, treat it as a leaf, reference module.
        } else {
            Module {
                file_name: format!("ref_{}.adoc", module_id),
                text: self.render(&module_id, tickets),
                included_modules: None,
            }
        }
    }

    fn matches_ticket(&self, ticket: &AbstractTicket) -> bool {
        let matches_doc_type = self
            .filter
            .doc_type
            .as_ref()
            .map_or(true, |dt| dt.contains(ticket.doc_type.as_ref().unwrap()));
        let matches_subsystem = self
            .filter
            .subsystem
            .as_ref()
            // TODO: Also take into account additional subsystems.
            .map_or(true, |sst| sst.contains(&ticket.subsystems[0]));
        let matches_component = self
            .filter
            .component
            .as_ref()
            // TODO: Also take into account additional components.
            .map_or(true, |c| c.contains(&ticket.components[0]));
        matches_doc_type && matches_subsystem && matches_component
    }
}

pub fn parse(template_file: &Path) -> Result<Template> {
    let text = fs::read_to_string(template_file).context("Cannot read the template file.")?;
    let templates: Template =
        serde_yaml::from_str(&text).context("Cannot parse the template file.")?;
    debug!("{:#?}", templates);
    Ok(templates)
}

pub fn format_document(tickets: &[AbstractTicket], template: &Template) -> Vec<Module> {
    let chapters: Vec<_> = template
        .chapters
        .iter()
        .map(|section| section.modules(tickets, None))
        .collect();
    debug!("Chapters: {:#?}", chapters);

    chapters
}
