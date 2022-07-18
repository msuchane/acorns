use std::collections::HashMap;

// use color_eyre::eyre::{Context, Result};

use crate::config::{Section, Template};
use crate::{extra_fields::DocTextStatus, ticket_abstraction::AbstractTicket};

/// The variant of the generated, output document:
///
/// * `Public`: The external variant intended for publishing the release notes.
/// * `Internal`: The debugging variant intended for preparing the release notes.
#[derive(PartialEq)]
pub enum DocumentVariant {
    Public,
    Internal,
}

/// The representation of a module, before being finally rendered.
#[derive(Clone, Debug, PartialEq)]
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
    fn render(
        &self,
        id: &str,
        tickets: &[AbstractTicket],
        variant: &DocumentVariant,
        ticket_stats: &mut HashMap<String, u32>,
    ) -> Option<String> {
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

        let matching_tickets: Vec<_> = variant_tickets
            .iter()
            .filter(|t| self.matches_ticket(t))
            .collect();

        // Record usage statistics for this leaf module
        for ticket in &matching_tickets {
            let counter = ticket_stats.entry(ticket.id.to_string()).or_insert(1);
            *counter += 1;
        }

        if matching_tickets.is_empty() {
            None
        } else {
            let heading = format!("= {}", &self.title);

            let release_notes: Vec<_> = matching_tickets
                .iter()
                .map(|t| t.release_note(variant))
                .collect();

            // If an introductory abstract is configured for this section, add it below the heading,
            // followed by a newline separator.
            let intro = if let Some(intro_abstract) = &self.intro_abstract {
                format!("{}\n", intro_abstract)
            } else {
                String::new()
            };

            Some(format!(
                "[id=\"{}\"]\n\
                {}\n\
                \n\
                {}\
                \n\
                {}\n",
                id,
                heading,
                intro,
                release_notes.join("\n\n")
            ))
        }
    }

    /// Convert the section into either a leaf module, or into an assembly and all
    /// the modules that it includes, recursively.
    ///
    /// Returns `None` if the module or assembly captured no release notes at all.
    fn modules(
        &self,
        tickets: &[AbstractTicket],
        prefix: Option<&str>,
        variant: &DocumentVariant,
        ticket_stats: &mut HashMap<String, u32>,
    ) -> Option<Module> {
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
                .filter_map(|s| {
                    s.modules(&matching_tickets, Some(&module_id), variant, ticket_stats)
                })
                .collect();
            // If the assembly receives no modules, because all its modules are empty, return None.
            if included_modules.is_empty() {
                None
            } else {
                let include_statements: Vec<String> = included_modules
                    .iter()
                    .map(|m| m.include_statement())
                    .collect();

                let include_block = include_statements.join("\n\n");

                // If an introductory abstract is configured for this section, add it below the heading,
                // followed by a newline separator.
                let intro = if let Some(intro_abstract) = &self.intro_abstract {
                    format!("{}\n", intro_abstract)
                } else {
                    String::new()
                };

                let text = format!(
                    "[id=\"{}\"]\n\
                    = {}\n\
                    \n\
                    {}\
                    \n\
                    {}\n",
                    &module_id, &self.title, intro, include_block
                );

                Some(Module {
                    file_name,
                    text,
                    included_modules: Some(included_modules),
                })
            }
        // If the section includes no sections, treat it as a leaf, reference module.
        } else {
            // If the module receives no release notes and its body is empty, return None.
            // Otherwise, return the module formatted with its release notes.
            self.render(&module_id, tickets, variant, ticket_stats)
                .map(|text| Module {
                    file_name: format!("ref_{}.adoc", module_id),
                    text,
                    included_modules: None,
                })
        }
    }

    /// Checks whether this section, with its filter configuration, can include a particular ticket.
    fn matches_ticket(&self, ticket: &AbstractTicket) -> bool {
        let matches_doc_type = match &self.filter.doc_type {
            Some(doc_types) => doc_types
                .iter()
                // Compare both doc types in lower case
                // TODO: Turn the `expect` into proper error handling. See also the other variables below.
                .any(|dt| dt.to_lowercase() == ticket.doc_type.to_lowercase()),
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

/// Form all modules that are recursively defined in the template configuration.
pub fn format_document(
    tickets: &[AbstractTicket],
    template: &Template,
    variant: &DocumentVariant,
) -> Vec<Module> {
    let mut ticket_stats = HashMap::new();

    for ticket in tickets.iter() {
        ticket_stats.insert(ticket.id.to_string(), 0);
    }

    // TODO: If no release notes trickle down into a chapter, the chapter is simply skipped.
    // However, includes from the manual RN content tend to target all chapters.
    // Figure out a solution. Perhaps an empty file to appease the include from outside?
    let chapters: Vec<_> = template
        .chapters
        .iter()
        .filter_map(|section| section.modules(tickets, None, variant, &mut ticket_stats))
        .collect();
    log::debug!("Chapters: {:#?}", chapters);

    // A crude way to ensure that the statistics are only printed once, and not twice.
    // TODO: Revisit, maybe return the value instead.
    if variant == &DocumentVariant::Internal {
        log::info!("Ticket usage statistics:\n{:#?}", ticket_stats);
    }

    chapters
}
