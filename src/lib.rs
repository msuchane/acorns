use std::fs;
use std::path::{Path, PathBuf};

use clap::ArgMatches;
use color_eyre::eyre::{Context, Result};

pub mod cli;
mod config;
mod logging;
mod note;
mod templating;
mod ticket_abstraction;

use config::tracker::Service;
use templating::Module;

/// Run the subcommand that the user picked on the command line.
pub fn run(cli_arguments: &ArgMatches) -> Result<()> {
    // Initialize the logging system based on the set verbosity
    let verbosity = *cli_arguments.get_one::<u8>("verbose").unwrap();
    logging::initialize_logger(verbosity)?;

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
fn display_single_ticket(ticket_args: &ArgMatches) -> Result<()> {
    log::info!("Downloading ticket information.");
    let service = match *ticket_args.get_one("service").unwrap() {
        "jira" => Service::Jira,
        "bugzilla" => Service::Bugzilla,
        _ => unreachable!(),
    };
    let ticket = ticket_abstraction::from_args(
        service,
        *ticket_args.get_one("id").unwrap(),
        *ticket_args.get_one("host").unwrap(),
        *ticket_args.get_one("api_key").unwrap(),
    )?;
    println!("{}", ticket.release_note());

    Ok(())
}

/// Run the `build` subcommand, which build the release notes project that's configured
/// in the project directory specified on the command line, or in the working directory.
fn build_rn_project(build_args: &ArgMatches) -> Result<()> {
    // By default, build release notes in the current working directory.
    let project_dir = match build_args.get_one::<PathBuf>("project") {
        Some(dir) => Path::new(dir),
        None => Path::new("."),
    };
    let abs_path = project_dir.canonicalize()?;

    log::info!("Building release notes in {}", abs_path.display());

    let tickets_path = abs_path.join("tickets.yaml");
    let trackers_path = abs_path.join("trackers.yaml");
    let templates_path = abs_path.join("templates.yaml");

    // TODO: Enable overriding the default config paths.
    // Record the paths to the configuration files.
    // The `value_of_os` method handles cases where a file name is nto valid UTF-8.
    // let tickets_path = Path::new(cli_arguments.value_of_os("tickets").unwrap());
    // let trackers_path = Path::new(cli_arguments.value_of_os("trackers").unwrap());

    log::debug!(
        "Configuration files:\n* {}\n* {}\n* {}",
        tickets_path.display(),
        trackers_path.display(),
        templates_path.display()
    );

    let modules = form_modules(&tickets_path, &trackers_path, &templates_path)?;

    log::info!("Saving the generated release notes.");
    write_rns(&modules, project_dir)?;

    log::info!("Done.");

    Ok(())
}

/// Prepare all populated and formatted modules that result from the RN project configuration.
fn form_modules(
    tickets_path: &Path,
    trackers_path: &Path,
    templates_path: &Path,
) -> Result<Vec<Module>> {
    // Parse the configuration files specified on the command line.
    let (tickets, trackers) = config::parse(tickets_path, trackers_path)?;
    let templates = templating::parse(templates_path)?;

    log::info!("Downloading ticket information.");
    let abstract_tickets = ticket_abstraction::from_queries(&tickets, &trackers)?;

    log::info!("Formatting the document.");
    Ok(templating::format_document(&abstract_tickets, &templates))
}

/// Write all the formatted RN modules as files to the output directory.
fn write_rns(modules: &[Module], out_dir: &Path) -> Result<()> {
    // By default, save the resulting document to the project directory under `generated/`.
    // TODO: Make the output configurable.
    let generated_dir = out_dir.join("generated");
    // Make sure that the output directory exists.
    fs::create_dir_all(&generated_dir)?;

    for module in modules {
        let out_file = &generated_dir.join(&module.file_name);
        log::debug!("Writing file: {}", out_file.display());
        fs::write(out_file, &module.text).context("Failed to write generated module.")?;

        // If the currently processed module is an assembly,
        // recursively descend into the assembly and write its included modules.
        if let Some(included_modules) = &module.included_modules {
            write_rns(included_modules, out_dir)?;
        }
    }

    Ok(())
}
