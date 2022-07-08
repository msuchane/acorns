use std::fs;
use std::path::Path;

use clap::ArgMatches;
use color_eyre::eyre::{bail, Context, Result};

pub mod cli;
mod config;
mod extra_fields;
mod logging;
mod note;
mod templating;
mod ticket_abstraction;

use config::tracker::Service;
use templating::Module;

/// The name of this program, as specified in Cargo.toml. Used later to access configuration files.
const PROGRAM_NAME: &str = env!("CARGO_PKG_NAME");

/// The sub-directory inside the release notes project that contains all Cizrna configuration and other files.
/// The name of this sub-directory is the same as the name of this program.
const DATA_PREFIX: &str = PROGRAM_NAME;

// TODO: Make the output configurable. Enable saving to a separate Git repository.
/// The sub-directory inside the data directory that contains all generated documents.
const GENERATED_PREFIX: &str = "generated";

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
fn display_single_ticket(ticket_args: &ArgMatches) -> Result<()> {
    log::info!("Downloading ticket information.");
    let service = match ticket_args.value_of("service").unwrap() {
        "jira" => Service::Jira,
        "bugzilla" => Service::Bugzilla,
        _ => unreachable!(),
    };
    let ticket = ticket_abstraction::from_args(
        service,
        ticket_args.value_of("id").unwrap(),
        ticket_args.value_of("host").unwrap(),
        ticket_args.value_of("api_key").unwrap(),
    )?;
    println!("{}", ticket.release_note());

    Ok(())
}

/// Run the `build` subcommand, which build the release notes project that's configured
/// in the project directory specified on the command line, or in the working directory.
fn build_rn_project(build_args: &ArgMatches) -> Result<()> {
    // By default, build release notes in the current working directory.
    let project_dir = match build_args.value_of_os("project") {
        Some(dir) => Path::new(dir),
        None => Path::new("."),
    };
    let abs_path = project_dir.canonicalize()?;
    let data_dir = abs_path.join(DATA_PREFIX);
    let generated_dir = data_dir.join(GENERATED_PREFIX);

    log::info!("Building release notes in {}", abs_path.display());

    // If not even the main configuration directory exists, exit with an error.
    if !data_dir.is_dir() {
        bail!(
            "The configuration directory is missing: {}",
            data_dir.display()
        );
    }

    // Prepare to access each configuration file.
    let tickets_path = data_dir.join("tickets.yaml");
    let trackers_path = data_dir.join("trackers.yaml");
    let templates_path = data_dir.join("templates.yaml");

    // TODO: Enable overriding the default config paths.
    //
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
    write_rns(&modules, &generated_dir)?;

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
fn write_rns(modules: &[Module], generated_dir: &Path) -> Result<()> {
    // Make sure that the output directory exists.
    fs::create_dir_all(&generated_dir)?;

    for module in modules {
        let out_file = &generated_dir.join(&module.file_name);
        log::debug!("Writing file: {}", out_file.display());
        fs::write(out_file, &module.text).context("Failed to write generated module.")?;

        // If the currently processed module is an assembly,
        // recursively descend into the assembly and write its included modules.
        if let Some(included_modules) = &module.included_modules {
            write_rns(included_modules, generated_dir)?;
        }
    }

    Ok(())
}
