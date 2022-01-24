use bugzilla_query;
use jira_query;
use std::path::Path;

mod cli;
mod config;
mod note;
mod ticket_abstraction;

fn main() {
    let cli_arguments = cli::arguments();

    if let Some(name) = cli_arguments.value_of("name") {
        println!("Value for name: {}", name);
    }

    let raw_tickets = cli_arguments.value_of_os("tickets").unwrap();
    let tickets_path = Path::new(raw_tickets);
    let raw_trackers = cli_arguments.value_of_os("trackers").unwrap();
    let trackers_path = Path::new(raw_trackers);
    eprintln!(
        "Configuration files: {}, {}",
        tickets_path.display(),
        trackers_path.display()
    );
    let (tickets, trackers) = config::parse(tickets_path, trackers_path);

    let mut release_notes: Vec<String> = Vec::new();

    for ticket in &tickets {
        match &ticket.tracker {
            config::TrackerType::Bugzilla => {
                println!("Bugzilla ticket: {:#?}", ticket);
                let bug = bugzilla_query::bug(
                    &trackers.bugzilla.host,
                    &ticket.key,
                    &trackers.bugzilla.api_key,
                );
                let rn = note::display_bugzilla_bug(&bug);
                release_notes.push(rn);
            }
            config::TrackerType::JIRA => {
                println!("JIRA ticket: {:#?}", ticket);
                let issue =
                    jira_query::issue(&trackers.jira.host, &ticket.key, &trackers.jira.api_key);
                let rn = note::display_jira_issue(&issue);
                release_notes.push(rn);
            }
        }
    }

    let document = release_notes.join("\n\n");

    println!("Release notes:\n\n{}", document);

    match cli_arguments.occurrences_of("debug") {
        0 => println!("Debug mode is off"),
        1 => println!("Debug mode is kind of on"),
        2 => println!("Debug mode is on"),
        _ => println!("Don't be crazy"),
    }

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
