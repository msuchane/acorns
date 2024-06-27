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

use std::collections::HashMap;
use std::rc::Rc;

use askama::Template;
//use color_eyre::Result;

use crate::config;
use crate::ticket_abstraction::AbstractTicket;
use crate::ticket_abstraction::TicketId;

/// A leaf, reference module that contains release notes with no further nesting.
#[derive(Template)]
#[template(path = "reference.adoc", escape = "none")]
struct Leaf<'a> {
    id: &'a str,
    title: &'a str,
    intro_abstract: &'a str,
    release_notes: &'a [String],
}

/// An assembly module that nests other assemblies or leaf reference modules.
#[derive(Template)]
#[template(path = "assembly.adoc", escape = "none")]
struct Assembly<'a> {
    id: &'a str,
    title: &'a str,
    intro_abstract: &'a str,
    includes: &'a [String],
}

/// The variant of the generated, output document:
///
/// * `External`: The external variant intended for publishing the release notes.
/// * `Internal`: The debugging variant intended for preparing the release notes.
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum DocumentVariant {
    External,
    Internal,
}

/// The representation of a module, before being finally rendered.
#[derive(Clone, Debug, PartialEq)]
pub enum Module {
    /// This is the full version of a module.
    WithContent {
        file_name: String,
        text: String,
        included_modules: Option<Vec<Self>>,
    },
    /// This is an outline of a module that only carries its file name.
    /// Its purpose is to create blank assemblies for top-level chapters.
    Blank {
        file_name: String,
    },
}

impl Module {
    /// The AsciiDoc include statement to include this module elsewhere.
    pub fn include_statement(&self) -> String {
        format!("include::{}[leveloffset=+1]", self.file_name())
    }
    /// The module's file name.
    pub fn file_name(&self) -> &str {
        match self {
            Self::Blank { file_name, .. } | Self::WithContent { file_name, .. } => file_name,
        }
    }
    /// Return `true` if the module is of the `WithContent` variant.
    fn has_content(&self) -> bool {
        match self {
            Self::WithContent { .. } => true,
            Self::Blank { .. } => false,
        }
    }
}

/// Convert a section title to an ID that's sanitized for AsciiDoc and HTML.
///
/// This function is taken from `newdoc` (<https://github.com/redhat-documentation/newdoc>).
fn id_fragment(title: &str) -> String {
    // The ID is all lower-case
    let mut title_with_replacements: String = title.to_lowercase();

    // Replace characters that aren't allowed in the ID, usually with a dash or an empty string
    let substitutions = [
        (" ", "-"),
        ("(", ""),
        (")", ""),
        ("?", ""),
        ("!", ""),
        ("'", ""),
        ("\"", ""),
        ("#", ""),
        ("%", ""),
        ("&", ""),
        ("*", ""),
        (",", "-"),
        (".", "-"),
        ("/", "-"),
        (":", "-"),
        (";", ""),
        ("@", "-at-"),
        ("\\", ""),
        ("`", ""),
        ("$", ""),
        ("^", ""),
        ("|", ""),
        ("=", "-"),
        // Remove known semantic markup from the ID:
        ("[package]", ""),
        ("[option]", ""),
        ("[parameter]", ""),
        ("[variable]", ""),
        ("[command]", ""),
        ("[replaceable]", ""),
        ("[filename]", ""),
        ("[literal]", ""),
        ("[systemitem]", ""),
        ("[application]", ""),
        ("[function]", ""),
        ("[gui]", ""),
        // Remove square brackets only after semantic markup:
        ("[", ""),
        ("]", ""),
        // TODO: Curly braces shouldn't appear in the title in the first place.
        // They'd be interpreted as attributes there.
        // Print an error in that case? Escape them with AsciiDoc escapes?
        ("{", ""),
        ("}", ""),
    ];

    // Perform all the defined replacements on the title
    for (old, new) in substitutions {
        title_with_replacements = title_with_replacements.replace(old, new);
    }

    // Replace remaining characters that aren't ASCII, or that are non-alphanumeric ASCII,
    // with dashes. For example, this replaces diacritics and typographic quotation marks.
    title_with_replacements = title_with_replacements
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();

    // Ensure the converted ID doesn't contain double dashes ("--"), because
    // that breaks references to the ID
    while title_with_replacements.contains("--") {
        title_with_replacements = title_with_replacements.replace("--", "-");
    }

    // Ensure that the ID doesn't end with a dash
    if title_with_replacements.ends_with('-') {
        let len = title_with_replacements.len();
        title_with_replacements = title_with_replacements[..len - 1].to_string();
    }

    title_with_replacements
}

impl config::Section {
    /// Convert the body of the section into AsciiDoc text that will serve
    /// as the body of the resulting module.
    fn render(
        &self,
        id: &str,
        tickets: &[&AbstractTicket],
        variant: DocumentVariant,
        with_priv_footnote: bool,
        ticket_stats: &mut HashMap<Rc<TicketId>, u32>,
    ) -> Option<String> {
        let matching_tickets: Vec<_> = tickets.iter().filter(|t| self.matches_ticket(t)).collect();

        // Record usage statistics for this leaf module
        for ticket in &matching_tickets {
            ticket_stats
                .entry(Rc::clone(&ticket.id))
                .and_modify(|counter| *counter += 1)
                .or_insert(1);
        }

        if matching_tickets.is_empty() {
            None
        } else {
            let release_notes: Vec<_> = matching_tickets
                .iter()
                .map(|t| t.release_note(variant, with_priv_footnote))
                .collect();

            let template = Leaf {
                id,
                title: &self.title,
                // If an introductory abstract is configured for this section, add it below the heading.
                intro_abstract: self.intro_abstract.as_ref().map_or("", |s| s.as_str()),
                release_notes: &release_notes,
            };

            Some(
                template
                    .render()
                    .expect("Failed to render a reference module template."),
            )
        }
    }

    /// Convert the section into either a leaf module, or into an assembly and all
    /// the modules that it includes, recursively.
    ///
    /// Returns `Blank` if the module or assembly captured no release notes at all.
    fn modules(
        &self,
        tickets: &[&AbstractTicket],
        prefix: Option<&str>,
        variant: DocumentVariant,
        with_priv_footnote: bool,
        ticket_stats: &mut HashMap<Rc<TicketId>, u32>,
    ) -> Module {
        let matching_tickets: Vec<&AbstractTicket> = tickets
            .iter()
            .filter(|&&t| self.matches_ticket(t))
            .copied()
            .collect();

        let module_id_fragment = id_fragment(&self.title);
        let module_id = if let Some(prefix) = prefix {
            format!("{prefix}-{module_id_fragment}")
        } else {
            module_id_fragment
        };

        // If the section includes other sections, treat it as an assembly.
        if let Some(sections) = &self.subsections {
            let file_name = format!("assembly_{module_id}.adoc");
            let included_modules: Vec<Module> = sections
                .iter()
                .map(|s| {
                    s.modules(
                        &matching_tickets,
                        Some(&module_id),
                        variant,
                        with_priv_footnote,
                        ticket_stats,
                    )
                })
                .filter(Module::has_content)
                .collect();
            // If the assembly receives no modules, because all its modules are empty, return Blank.
            if included_modules.is_empty() {
                Module::Blank { file_name }
            } else {
                let include_statements: Vec<String> = included_modules
                    .iter()
                    .map(Module::include_statement)
                    .collect();

                let template = Assembly {
                    id: &module_id,
                    title: &self.title,
                    // If an introductory abstract is configured for this section, add it below the heading.
                    intro_abstract: self.intro_abstract.as_ref().map_or("", |s| s.as_str()),
                    includes: &include_statements,
                };

                let text = template
                    .render()
                    .expect("Failed to render an assembly template.");

                Module::WithContent {
                    file_name,
                    text,
                    included_modules: Some(included_modules),
                }
            }
        // If the section includes no sections, treat it as a leaf, reference module.
        } else {
            // If the module receives no release notes and its body is empty, return Blank.
            // Otherwise, return the module formatted with its release notes.
            let text = self.render(
                &module_id,
                tickets,
                variant,
                with_priv_footnote,
                ticket_stats,
            );
            let file_name = format!("ref_{module_id}.adoc");
            if let Some(text) = text {
                Module::WithContent {
                    file_name,
                    text,
                    included_modules: None,
                }
            } else {
                Module::Blank { file_name }
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
                .any(|dt| dt.to_lowercase() == ticket.doc_type.to_lowercase()),
            // If the filter doesn't configure a doc type, match by default
            None => true,
        };
        let matches_subsystem = match &self.filter.subsystem {
            Some(ssts) => {
                // Try to unwrap the result of the subsystems field only when a configured filter
                // actually needs the subsystems. That way, subsystems are strictly optional,
                // and if a project doesn't configure them at all, the release notes build
                // can still finish successfully.
                //
                // TODO: Consider using a proper `Result` chain here instead of simply panicking.
                let unwrapped_ssts = match &ticket.subsystems {
                    Ok(ssts) => ssts,
                    // If subsystems resulted in an error, print out some debugging information
                    // before quitting. The ticket ID is especially useful.
                    Err(e) => {
                        log::error!("Invalid subsystems field in ticket {}.", &ticket.id);
                        panic!("{}", e);
                    }
                };

                ssts.iter()
                    // Compare both subsystems in lower case.
                    // Match if any of the ticket SSTs matches any of the template SSTs.
                    .any(|sst| {
                        unwrapped_ssts
                            .iter()
                            .any(|ticket_sst| sst.to_lowercase() == ticket_sst.to_lowercase())
                    })
            }
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
    tickets: &[&AbstractTicket],
    template: &config::Template,
    variant: DocumentVariant,
    with_priv_footnote: bool,
) -> Vec<Module> {
    // Prepare a container for ticket usage statistics.
    let mut ticket_stats = HashMap::new();

    // Initialize every ticket in the statistics with 0 usage.
    // Later, the number increases each time that the ticket is used.
    // Initializing with 0 rather than relying on each ticket's `entry` call
    // is necessary for tickets that end up unused, because they wouldn't
    // call `entry` at all, and would report nothing.
    for ticket in tickets {
        ticket_stats.insert(Rc::clone(&ticket.id), 0);
    }

    // TODO: If no release notes trickle down into a chapter, the chapter is simply skipped.
    // However, includes from the manual RN content tend to target all chapters.
    // Figure out a solution. Perhaps an empty file to appease the include from outside?
    let chapters: Vec<_> = template
        .chapters
        .iter()
        .map(|section| {
            section.modules(
                tickets,
                None,
                variant,
                with_priv_footnote,
                &mut ticket_stats,
            )
        })
        .collect();
    log::debug!("Chapters: {:#?}", chapters);

    // A crude way to ensure that the statistics are only printed once, and not twice.
    // TODO: Revisit, maybe return the value instead.
    if variant == DocumentVariant::Internal {
        report_usage_statistics(&ticket_stats);
    }

    chapters
}

/// Log statistics about tickets that haven't been used anywhere in the templates,
/// or have been used more than once. Log both as warnings.
fn report_usage_statistics(ticket_stats: &HashMap<Rc<TicketId>, u32>) {
    let unused: Vec<String> = ticket_stats
        .iter()
        .filter(|&(_k, &v)| v == 0)
        .map(|(k, _v)| Rc::clone(k).to_string())
        .collect();

    let overused: Vec<String> = ticket_stats
        .iter()
        .filter(|&(_k, &v)| v > 1)
        .map(|(k, _v)| Rc::clone(k).to_string())
        .collect();

    if !unused.is_empty() {
        log::warn!("Tickets unused in the templates:\n\t {}", unused.join(", "));
    }

    if !overused.is_empty() {
        log::warn!(
            "Tickets used more than once in the templates:\n\t {}",
            overused.join(", ")
        );
    }
}
