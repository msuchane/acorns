use log::{debug, info};
use std::path::Path;

mod cli;
mod config;
mod logging;
mod note;
mod ticket_abstraction;

fn main() {
    let cli_arguments = cli::arguments();

    // Initialize the logging system based on the set verbosity
    logging::initialize_logger(cli_arguments.occurrences_of("verbose"));

    // Record the paths to the configuration files.
    // The `value_of_os` method handles cases where a file name is nto valid UTF-8.
    let tickets_path = Path::new(cli_arguments.value_of_os("tickets").unwrap());
    let trackers_path = Path::new(cli_arguments.value_of_os("trackers").unwrap());
    debug!(
        "Configuration files: {}, {}",
        tickets_path.display(),
        trackers_path.display()
    );

    // Parse the configuration files specified on the command line.
    let (tickets, trackers) = config::parse(tickets_path, trackers_path);

    let abstract_tickets = ticket_abstraction::from_queries(&tickets, &trackers);

    let release_notes: Vec<String> = abstract_tickets
        .into_iter()
        .map(|t| t.release_note())
        .collect();
    let document = release_notes.join("\n\n");

    info!("Release notes:\n\n{}", document);

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
}
