use std::path::Path;

mod bugzilla_query;
mod cli;
mod config;
mod jira_query;

fn main() {
    println!("Hello, world!");

    let cli_arguments = cli::arguments();

    if let Some(name) = cli_arguments.value_of("name") {
        println!("Value for name: {}", name);
    }

    let raw_config = cli_arguments.value_of_os("config").unwrap();
    let config_path = Path::new(raw_config);
    let raw_trackers = cli_arguments.value_of_os("trackers").unwrap();
    let trackers_path = Path::new(raw_trackers);
    println!(
        "Configuration files: {}, {}",
        config_path.display(),
        trackers_path.display()
    );
    let (tickets, trackers) = config::get(config_path, trackers_path);

    for ticket in &tickets {
        match &ticket.tracker {
            config::TrackerType::Bugzilla => {
                println!("Bugzilla ticket: {:#?}", ticket);
                let _bugs = bugzilla_query::main(
                    &trackers.bugzilla.host,
                    &ticket.key,
                    &trackers.bugzilla.api_key,
                );
            }
            config::TrackerType::JIRA => {
                println!("JIRA ticket: {:#?}", ticket);
                let _issue =
                    jira_query::main(&trackers.jira.host, &ticket.key, &trackers.jira.api_key);
            }
        }
    }

    match cli_arguments.occurrences_of("debug") {
        0 => println!("Debug mode is off"),
        1 => println!("Debug mode is kind of on"),
        2 => println!("Debug mode is on"),
        _ => println!("Don't be crazy"),
    }

    if let Some(cli_arguments) = cli_arguments.subcommand_matches("jira") {
        let _issue = jira_query::main(
            cli_arguments.value_of("server").unwrap(),
            cli_arguments.value_of("ticket").unwrap(),
            cli_arguments.value_of("api_key").unwrap(),
        );
    }
    if let Some(cli_arguments) = cli_arguments.subcommand_matches("bugzilla") {
        let _bugs = bugzilla_query::main(
            cli_arguments.value_of("server").unwrap(),
            cli_arguments.value_of("ticket").unwrap(),
            cli_arguments.value_of("api_key").unwrap(),
        );
    }
}
