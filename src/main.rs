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

fn main() -> Result<()> {
    let cli_arguments = cli::arguments();
        run(&cli_arguments)?;

    Ok(())
}

fn run(cli_arguments: &ArgMatches) -> Result<()> {
    // Initialize the logging system based on the set verbosity
    logging::initialize_logger(cli_arguments.occurrences_of("verbose"));

    if let Some(cli_arguments) = cli_arguments.subcommand_matches("jira") {
        let _issue = jira_query::issue(
            cli_arguments.value_of("server").unwrap(),
            cli_arguments.value_of("ticket").unwrap(),
            cli_arguments.value_of("api_key").unwrap(),
        );
    }
    if let Some(cli_arguments) = cli_arguments.subcommand_matches("bugzilla") {
        let _bugs = bugzilla_query::bug(
            cli_arguments.value_of("server").unwrap(),
            cli_arguments.value_of("ticket").unwrap(),
            cli_arguments.value_of("api_key").unwrap(),
        );
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

        write_rns(&tickets_path, &trackers_path)?;
    }

    Ok(())
}

fn write_rns(tickets_path: &Path, trackers_path: &Path) -> Result<()> {
    debug!(
        "Configuration files: {}, {}",
        tickets_path.display(),
        trackers_path.display()
    );

    // Parse the configuration files specified on the command line.
    let (tickets, trackers) = config::parse(tickets_path, trackers_path)?;

    let abstract_tickets = ticket_abstraction::from_queries(&tickets, &trackers)?;

    let release_notes: Vec<String> = abstract_tickets
        .into_iter()
        .map(|t| t.release_note())
        .collect();
    let document = format!("= Release notes\n\n{}", release_notes.join("\n\n"));

    debug!("Release notes:\n\n{}", document);

    let out_file = Path::new("main.adoc");
    fs::write(out_file, document)?;

    Ok(())
}
