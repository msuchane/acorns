use std::fs;
use std::path::Path;

use clap::ArgMatches;
use color_eyre::eyre::Result;
use log::{debug, info};

mod cli;
mod config;
mod logging;
mod note;
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

    if let Some(cli_arguments) = cli_arguments.subcommand_matches("jira") {
        let ticket = ticket_abstraction::from_args(
            Service::Jira,
            cli_arguments.value_of("ticket").unwrap(),
            cli_arguments.value_of("server").unwrap(),
            cli_arguments.value_of("api_key").unwrap(),
        )?;
        info!("{}", ticket.release_note());
    }
    if let Some(cli_arguments) = cli_arguments.subcommand_matches("bugzilla") {
        let ticket = ticket_abstraction::from_args(
            Service::Bugzilla,
            cli_arguments.value_of("ticket").unwrap(),
            cli_arguments.value_of("server").unwrap(),
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

        // TODO: Enable overriding the default config paths.
        // Record the paths to the configuration files.
        // The `value_of_os` method handles cases where a file name is nto valid UTF-8.
        // let tickets_path = Path::new(cli_arguments.value_of_os("tickets").unwrap());
        // let trackers_path = Path::new(cli_arguments.value_of_os("trackers").unwrap());

        debug!(
            "Configuration files: {}, {}",
            tickets_path.display(),
            trackers_path.display()
        );

        // Parse the configuration files specified on the command line.
        let (tickets, trackers) = config::parse(&tickets_path, &trackers_path)?;

        let abstract_tickets = ticket_abstraction::from_queries(&tickets, &trackers)?;

        let release_notes: Vec<String> = abstract_tickets
            .into_iter()
            .map(|t| t.release_note())
            .collect();

        write_rns(&release_notes, project_dir)?;
    }

    Ok(())
}

fn write_rns(release_notes: &[String], out_dir: &Path) -> Result<()> {
    let document = format!("= Release notes\n\n{}", release_notes.join("\n\n"));

    debug!("Release notes:\n\n{}", document);

    // By default, save the resulting document to the project directory.
    // TODO: Make the output configurable.
    let out_file = out_dir.join("main.adoc");
    fs::write(out_file, document)?;

    Ok(())
}
