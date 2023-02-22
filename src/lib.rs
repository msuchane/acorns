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

// Enable additional clippy lints by default.
#![warn(
    clippy::pedantic,
    clippy::unwrap_used,
    clippy::clone_on_ref_ptr,
    clippy::todo
)]
// Disable the documentation clippy lint, so that it stops suggesting backticks around AsciiDoc.
#![allow(clippy::doc_markdown)]
// Forbid unsafe code in this program.
#![forbid(unsafe_code)]

use std::fs;
use std::path::Path;

use color_eyre::eyre::{Result, WrapErr};

pub mod cli;
mod config;
mod convert;
mod extra_fields;
mod init;
mod logging;
mod note;
mod references;
mod status_report;
mod summary_list;
mod templating;
mod ticket_abstraction;
mod tracker_access;

use cli::{Cli, Commands};

// use config::tracker::Service;
use templating::{DocumentVariant, Module};

use crate::config::Project;
pub use crate::ticket_abstraction::AbstractTicket;

/// Run the subcommand that the user picked on the command line.
pub fn run(cli: &Cli) -> Result<()> {
    // Initialize the logging system based on the set verbosity
    logging::initialize_logger(cli.verbose)?;

    match &cli.command {
        // If the user picked the `build` subcommand, build the specified release notes project directory
        Commands::Build { project } => {
            build_rn_project(project)?;
        }
        // If the user picked the `ticket` subcommand, fetch and display a single ticket
        Commands::Ticket { .. } => {
            display_single_ticket()?;
        }
        // If the user picked the `convert` subcommand, convert from the CoRN 3 config file
        Commands::Convert {
            legacy_config,
            new_config,
        } => {
            convert::convert(legacy_config, new_config)?;
        }
        Commands::Init { directory } => init::initialize_directory(directory)
            .wrap_err("Failed to initialize the project directory.")?,
    }

    Ok(())
}

/// Run the `ticket` subcommand, which downloads information about the single specified ticket
/// and prints out the release note resulting from the ticket.
fn display_single_ticket() -> Result<()> {
    // TODO: Tie in the ticket subcommand with the new tracker configuration.
    todo!();
    /*
    log::info!("Downloading ticket information.");
    let service = match ticket_args.value_of("service").unwrap() {
        "jira" => Service::Jira,
        "bugzilla" => Service::Bugzilla,
        _ => unreachable!(),
    };

    let _ticket = tracker_access::ticket(
        ticket_args.value_of("id").unwrap(),
        ticket_args.value_of("api_key").unwrap(),
        service,
        todo!(),
    )?;

    let variant = DocumentVariant::Internal;
    println!("{}", ticket.release_note(&variant));

    Ok(())
    */
}

/// Run the `build` subcommand, which build the release notes project that's configured
/// in the project directory specified on the command line, or in the working directory.
fn build_rn_project(project_dir: &Path) -> Result<()> {
    // TODO: Recognize the optional paths to different config files.
    let project = Project::new(project_dir)?;

    log::info!("Building release notes in {}", &project.base_dir.display());

    let document = Document::new(&project)?;

    document.write_variants(&project.generated_dir)?;

    Ok(())
}

/// Holds all the data generated from the project configuration before writing them to disk.
struct Document {
    internal_modules: Vec<Module>,
    external_modules: Vec<Module>,
    status_table: String,
    json_status: String,
    internal_summary: String,
    external_summary: String,
}

impl Document {
    /// Prepare all populated and formatted modules that result from the RN project configuration.
    /// Returns a tuple with the document generated in two variants: (Internal, External).
    fn new(project: &Project) -> Result<Self> {
        let abstract_tickets =
            ticket_abstraction::from_queries(&project.tickets, &project.trackers)?;

        // Filter internal and external tickets here before formatting the document.
        // That way, functions in `templating` don't have to keep checking if they're
        // working on the right ticket subset.
        let tickets_for_internal = variant_tickets(&abstract_tickets, DocumentVariant::Internal);
        let tickets_for_external = variant_tickets(&abstract_tickets, DocumentVariant::External);

        let internal_modules = templating::format_document(
            &tickets_for_internal,
            &project.templates,
            DocumentVariant::Internal,
        );
        let external_modules = templating::format_document(
            &tickets_for_external,
            &project.templates,
            DocumentVariant::External,
        );

        let (status_table, json_status) = status_report::analyze_status(&abstract_tickets)?;

        let internal_summary =
            summary_list::appendix(&tickets_for_internal, DocumentVariant::Internal)?;
        let external_summary =
            summary_list::appendix(&tickets_for_external, DocumentVariant::External)?;

        Ok(Self {
            internal_modules,
            external_modules,
            status_table,
            json_status,
            internal_summary,
            external_summary,
        })
    }

    /// Write the formatted RN modules of a document variant as files to the output directory.
    fn write_variant(modules: &[Module], summary: &str, generated_dir: &Path) -> Result<()> {
        // Make sure that the output directory exists.
        fs::create_dir_all(generated_dir)?;

        for module in modules {
            let out_file = generated_dir.join(&module.file_name);
            log::debug!("Writing file: {}", out_file.display());
            fs::write(out_file, &module.text).wrap_err("Failed to write generated module.")?;

            // If the currently processed module is an assembly,
            // recursively descend into the assembly and write its included modules.
            if let Some(included_modules) = &module.included_modules {
                Self::write_variant(included_modules, summary, generated_dir)?;
            }

            // Save the appendix.
            let summary_file = generated_dir.join("ref_list-of-tickets-by-component.adoc");
            log::debug!("Writing file: {}", summary_file.display());
            fs::write(summary_file, summary)
                .wrap_err("Failed to write generated summary appendix.")?;
        }

        Ok(())
    }

    /// Write the formatted RN modules of both document variants as files to the output directory.
    fn write_variants(&self, generated_dir: &Path) -> Result<()> {
        log::info!("Saving the generated release notes.");

        // Remove all previously generated content so that it doesn't interfere with the new build.
        if generated_dir.exists() {
            fs::remove_dir_all(generated_dir)?;
        }

        let internal_dir = generated_dir.join("internal");
        let external_dir = generated_dir.join("external");

        // Save the newly generated files.
        Self::write_variant(
            &self.internal_modules,
            &self.internal_summary,
            &internal_dir,
        )?;
        Self::write_variant(
            &self.external_modules,
            &self.external_summary,
            &external_dir,
        )?;

        // Save the status table.
        let html_status_file = generated_dir.join("status-table.html");
        log::debug!("Writing file: {}", html_status_file.display());
        fs::write(html_status_file, &self.status_table)
            .wrap_err("Failed to write the status table.")?;

        // Save the JSON status.
        let json_status_file = generated_dir.join("status-table.json");
        log::debug!("Writing file: {}", json_status_file.display());
        fs::write(json_status_file, &self.json_status)
            .wrap_err("Failed to write the JSON status.")?;

        Ok(())
    }
}

/// Select only those tickets that belong in the Internal or External variant.
fn variant_tickets(
    all_tickets: &[AbstractTicket],
    variant: DocumentVariant,
) -> Vec<&AbstractTicket> {
    match variant {
        // The internal variant accepts all tickets.
        DocumentVariant::Internal => all_tickets.iter().collect(),
        // The external variant accepts only finished and approved tickets.
        DocumentVariant::External => all_tickets
            .iter()
            .filter(|t| t.doc_text_status == extra_fields::DocTextStatus::Approved)
            .collect(),
    }
}
