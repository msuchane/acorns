use bugzilla_query;
use jira_query;
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

    if let Some(name) = cli_arguments.value_of("name") {
        debug!("Value for name: {}", name);
    }

    let raw_tickets = cli_arguments.value_of_os("tickets").unwrap();
    let tickets_path = Path::new(raw_tickets);
    let raw_trackers = cli_arguments.value_of_os("trackers").unwrap();
    let trackers_path = Path::new(raw_trackers);
    debug!(
        "Configuration files: {}, {}",
        tickets_path.display(),
        trackers_path.display()
    );
    let (tickets, trackers) = config::parse(tickets_path, trackers_path);

    let mut release_notes: Vec<String> = Vec::new();

    for ticket in &tickets {
        match &ticket.tracker {
            config::TrackerType::Bugzilla => {
                debug!("Bugzilla ticket: {:#?}", ticket);
                let bug = bugzilla_query::bug(
                    &trackers.bugzilla.host,
                    &ticket.key,
                    &trackers.bugzilla.api_key,
                );
                let rn = note::display_bugzilla_bug(&bug);
                release_notes.push(rn);
            }
            config::TrackerType::Jira => {
                debug!("Jira ticket: {:#?}", ticket);
                let issue =
                    jira_query::issue(&trackers.jira.host, &ticket.key, &trackers.jira.api_key);
                let rn = note::display_jira_issue(&issue);
                release_notes.push(rn);
            }
        }
    }

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
