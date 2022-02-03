use std::fs;
use std::path::Path;

use clap::ArgMatches;
use color_eyre::eyre::Result;
use log::{debug, info};

mod cli;
mod config;
mod logging;
mod note;
mod templating;
mod ticket_abstraction;

use config::tracker::Service;

fn main() -> Result<()> {
    let cli_arguments = cli::arguments();
    run(&cli_arguments)?;

    Ok(())
}

fn run(cli_arguments: &ArgMatches) -> Result<()> {
    // Initialize the logging system based on the set verbosity
    logging::initialize_logger(cli_arguments.occurrences_of("verbose"));

    if let Some(cli_arguments) = cli_arguments.subcommand_matches("ticket") {
        let service = match cli_arguments.value_of("service").unwrap() {
            "jira" => Service::Jira,
            "bugzilla" => Service::Bugzilla,
            _ => unreachable!(),
        };
        let ticket = ticket_abstraction::from_args(
            service,
            cli_arguments.value_of("id").unwrap(),
            cli_arguments.value_of("host").unwrap(),
            cli_arguments.value_of("api_key").unwrap(),
        )?;
        info!("{}", ticket.release_note());
    }

    if let Some(build) = cli_arguments.subcommand_matches("build") {
        // By default, build release notes in the current working directory.
        let project_dir = match build.value_of_os("project") {
            Some(dir) => Path::new(dir),
            None => Path::new("."),
        };
        let tickets_path = project_dir.join("tickets.yaml");
        let trackers_path = project_dir.join("trackers.yaml");
        let templates_path = project_dir.join("templates.yaml");

        // TODO: Enable overriding the default config paths.
        // Record the paths to the configuration files.
        // The `value_of_os` method handles cases where a file name is nto valid UTF-8.
        // let tickets_path = Path::new(cli_arguments.value_of_os("tickets").unwrap());
        // let trackers_path = Path::new(cli_arguments.value_of_os("trackers").unwrap());

        debug!(
            "Configuration files: {}, {}, {}",
            tickets_path.display(),
            trackers_path.display(),
            templates_path.display()
        );

        // Parse the configuration files specified on the command line.
        let (tickets, trackers) = config::parse(&tickets_path, &trackers_path)?;
        let templates = templating::parse(&templates_path)?;

        let abstract_tickets = ticket_abstraction::from_queries(&tickets, &trackers)?;

        let document = templating::format_document(&abstract_tickets, templates);

        write_rns(&document, project_dir)?;
    }

    Ok(())
}

fn write_rns(document: &str, out_dir: &Path) -> Result<()> {
    // By default, save the resulting document to the project directory.
    // TODO: Make the output configurable.
    let out_file = out_dir.join("main.adoc");
    fs::write(out_file, document)?;

    Ok(())
}
