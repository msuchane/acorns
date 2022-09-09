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
    clippy::expect_used,
    clippy::clone_on_ref_ptr,
    clippy::todo
)]
// Disable the documentation clippy lint, so that it stops suggesting backticks around AsciiDoc.
#![allow(clippy::doc_markdown)]
// Forbid unsafe code in this program.
#![forbid(unsafe_code)]

use std::fs;
use std::path::Path;

use clap::ArgMatches;
use color_eyre::eyre::{Result, WrapErr};

pub mod cli;
mod config;
mod extra_fields;
mod logging;
mod note;
mod references;
mod status_report;
mod templating;
mod ticket_abstraction;
mod tracker_access;

// use config::tracker::Service;
use templating::{DocumentVariant, Module};

use crate::config::Project;

/// Run the subcommand that the user picked on the command line.
pub fn run(cli_arguments: &ArgMatches) -> Result<()> {
    // Initialize the logging system based on the set verbosity
    logging::initialize_logger(cli_arguments.occurrences_of("verbose"))?;

    // If the user picked the `ticket` subcommand, fetch and display a single ticket
    if let Some(cli_arguments) = cli_arguments.subcommand_matches("ticket") {
        display_single_ticket(cli_arguments)?;
    }

    // If the user picked the `build` subcommand, build the specified release notes project directory
    if let Some(build_args) = cli_arguments.subcommand_matches("build") {
        build_rn_project(build_args)?;
    }

    Ok(())
}

/// Run the `ticket` subcommand, which downloads information about the single specified ticket
/// and prints out the release note resulting from the ticket.
fn display_single_ticket(_ticket_args: &ArgMatches) -> Result<()> {
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
fn build_rn_project(build_args: &ArgMatches) -> Result<()> {
    // By default, build release notes in the current working directory.
    let project_dir = match build_args.value_of_os("project") {
        Some(dir) => Path::new(dir),
        None => Path::new("."),
    };

    let project = Project::new(project_dir)?;

    log::info!("Building release notes in {}", &project.base_dir.display());

    let document = Document::new(&project)?;

    document.write_variants(&project.generated_dir)?;

    Ok(())
}

/// Holds all the data generated from the project configuration before writing them to disk.
struct Document {
    internal: Vec<Module>,
    public: Vec<Module>,
    status_table: String,
}

impl Document {
    /// Prepare all populated and formatted modules that result from the RN project configuration.
    /// Returns a tuple with the document generated in two variants: (Internal, Public).
    fn new(project: &Project) -> Result<Self> {
        let abstract_tickets =
            ticket_abstraction::from_queries(&project.tickets, &project.trackers)?;

        let internal = templating::format_document(
            &abstract_tickets,
            &project.templates,
            &DocumentVariant::Internal,
        );
        let public = templating::format_document(
            &abstract_tickets,
            &project.templates,
            &DocumentVariant::Public,
        );

        let status_table = status_report::analyze_status(&abstract_tickets)?;

        Ok(Self {
            internal,
            public,
            status_table,
        })
    }

    /// Write the formatted RN modules of a document variant as files to the output directory.
    fn write_variant(modules: &[Module], generated_dir: &Path) -> Result<()> {
        // Make sure that the output directory exists.
        fs::create_dir_all(&generated_dir)?;

        for module in modules {
            let out_file = &generated_dir.join(&module.file_name);
            log::debug!("Writing file: {}", out_file.display());
            fs::write(out_file, &module.text).wrap_err("Failed to write generated module.")?;

            // If the currently processed module is an assembly,
            // recursively descend into the assembly and write its included modules.
            if let Some(included_modules) = &module.included_modules {
                Self::write_variant(included_modules, generated_dir)?;
            }
        }

        Ok(())
    }

    /// Write the formatted RN modules of both document variants as files to the output directory.
    fn write_variants(&self, generated_dir: &Path) -> Result<()> {
        log::info!("Saving the generated release notes.");

        // Remove all previously generated content so that it doesn't interfere with the new build.
        if generated_dir.exists() {
            fs::remove_dir_all(&generated_dir)?;
        }

        let internal_dir = generated_dir.join("internal");
        let public_dir = generated_dir.join("public");

        // Save the newly generated files.
        Self::write_variant(&self.internal, &internal_dir)?;
        Self::write_variant(&self.public, &public_dir)?;

        // Save the status table.
        let status_file = generated_dir.join("status-table.html");
        log::debug!("Writing file: {}", status_file.display());
        fs::write(status_file, &self.status_table).wrap_err("Failed to write generated module.")?;

        Ok(())
    }
}
