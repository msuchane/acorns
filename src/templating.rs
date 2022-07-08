use std::fs;
use std::path::Path;

use color_eyre::eyre::{Context, Result};
use serde::Deserialize;

use crate::{extra_fields::DocTextStatus, ticket_abstraction::AbstractTicket};

/// This struct models the template configuration file.
/// It includes both `chapters` and `sections` because this is a way
/// in YaML to create reusable section definitions that can then
/// appear several times in different places. They have to be defined
/// on the top level, outside the actual chapters.
#[derive(Debug, PartialEq, Deserialize)]
pub struct Template {
    pub chapters: Vec<Section>,
    pub sections: Option<Vec<Section>>,
}

/// This struct covers the necessary properties of a section, which can either
/// turn into a module if it's a leaf, or into an assembly if it includes
/// more sections.
///
/// The `filter` field narrows down the tickets that can appear in this module
/// or in the modules that are included in this assembly.
#[derive(Debug, PartialEq, Deserialize)]
pub struct Section {
    pub title: String,
    pub filter: Filter,
    pub sections: Option<Vec<Section>>,
}

/// The configuration of a filter, which narrows down the tickets
/// that can appear in the section that the filter belongs to.
#[derive(Debug, PartialEq, Deserialize)]
pub struct Filter {
    pub doc_type: Option<Vec<String>>,
    pub subsystem: Option<Vec<String>>,
    pub component: Option<Vec<String>>,
}

/// The variant of the generated, output document:
///
/// * `Public`: The external variant intended for publishing teh release notes.
/// * `Internal`: The debugging variant intended for preparing the release notes.
#[derive(PartialEq)]
pub enum DocumentVariant {
    Public,
    Internal,
}

/// The representation of a module, before being finally rendered.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Module {
    pub file_name: String,
    pub text: String,
    pub included_modules: Option<Vec<Self>>,
}

impl Module {
    /// The AsciiDoc include statement to include this module elsewhere.
    pub fn include_statement(&self) -> String {
        format!("include::{}[leveloffset=+1]", &self.file_name)
    }
}

impl Section {
    /// Convert the body of the section into AsciiDoc text that will serve
    /// as the body of the resulting module.
    fn render(&self, id: &str, tickets: &[AbstractTicket], variant: &DocumentVariant) -> String {
        let heading = format!("= {}", &self.title);

        // Select only those tickets that belong in the Internal or Public variant.
        let variant_tickets: Vec<&AbstractTicket> = match variant {
            // The internal variant accepts all tickets.
            DocumentVariant::Internal => tickets.iter().collect(),
            // The public variant accepts only finished and approved tickets.
            DocumentVariant::Public => tickets
                .iter()
                .filter(|t| t.doc_text_status == DocTextStatus::Approved)
                .collect(),
        };

        let matching_tickets = variant_tickets.iter().filter(|t| self.matches_ticket(t));
        let release_notes: Vec<_> = matching_tickets.map(|t| t.release_note(variant)).collect();
        format!(
            "[id=\"{}\"]\n\
            {}\n\
            \n\
            {}",
            id,
            heading,
            release_notes.join("\n\n")
        )
    }

    /// Convert the section into either a leaf module, or into an assembly and all
    /// the modules that it includes, recursively.
    fn modules(
        &self,
        tickets: &[AbstractTicket],
        prefix: Option<&str>,
        variant: &DocumentVariant,
    ) -> Module {
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
                .map(|s| s.modules(&matching_tickets, Some(&module_id), variant))
                .collect();
            let include_statements: Vec<String> = included_modules
                .iter()
                .map(|m| m.include_statement())
                .collect();
            let include_block = include_statements.join("\n\n");
            let text = format!(
                "[id=\"{}\"]\n\
                = {}\n\
                \n\
                {}",
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
                text: self.render(&module_id, tickets, variant),
                included_modules: None,
            }
        }
    }

    /// Checks whether this section, with its filter configuration, can include a particular ticket.
    fn matches_ticket(&self, ticket: &AbstractTicket) -> bool {
        let matches_doc_type = match &self.filter.doc_type {
            Some(doc_types) => doc_types
                .iter()
                // Compare both doc types in lower case
                // TODO: Turn the `expect` into proper error handling. See also the other variables below.
                .any(|dt| {
                    dt.to_lowercase()
                        == ticket
                            .doc_type
                            .as_ref()
                            .expect("Ticket has no doc type.")
                            .to_lowercase()
                }),
            // If the filter doesn't configure a doc type, match by default
            None => true,
        };
        let matches_subsystem = match &self.filter.subsystem {
            Some(ssts) => ssts
                .iter()
                // Compare both subsystems in lower case.
                // Match if any of the ticket SSTs matches any of the template SSTs.
                .any(|sst| {
                    ticket
                        .subsystems
                        .iter()
                        .any(|ticket_sst| sst.to_lowercase() == ticket_sst.to_lowercase())
                }),
            // If the filter doesn't configure a subsystem, match by default
            None => true,
        };
        let matches_component = match &self.filter.component {
            Some(components) => components
                .iter()
                // Compare both components in lower case
                // Match if any of the ticket SSTs matches any of the template SSTs.
                .any(|cmp| {
                    ticket
                        .components
                        .iter()
                        .any(|ticket_cmp| cmp.to_lowercase() == ticket_cmp.to_lowercase())
                }),
            // If the filter doesn't configure a component, match by default
            None => true,
        };

        matches_doc_type && matches_subsystem && matches_component
    }
}

/// Parse the template configuration files into template structs, with chapter and section definitions.
pub fn parse(template_file: &Path) -> Result<Template> {
    let text = fs::read_to_string(template_file).context("Cannot read the template file.")?;
    let templates: Template =
        serde_yaml::from_str(&text).context("Cannot parse the template file.")?;
    log::debug!("{:#?}", templates);
    Ok(templates)
}

/// Form all modules that are recursively defined in the template configuration.
pub fn format_document(
    tickets: &[AbstractTicket],
    template: &Template,
    variant: &DocumentVariant,
) -> Vec<Module> {
    let chapters: Vec<_> = template
        .chapters
        .iter()
        .map(|section| section.modules(tickets, None, variant))
        .collect();
    log::debug!("Chapters: {:#?}", chapters);

    chapters
}
