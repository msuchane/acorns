use std::path::Path;

mod bugzilla_query;
mod cli;
mod jira_query;
use crate::jira_query::JiraIssue;

fn main() {
    println!("Hello, world!");

    let cli_arguments = cli::build_cli();

    if let Some(name) = cli_arguments.value_of("name") {
        println!("Value for name: {}", name);
    }

    if let Some(raw_config) = cli_arguments.value_of_os("config") {
        let config_path = Path::new(raw_config);
        println!("Value for config: {}", config_path.display());
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
