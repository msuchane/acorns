use clap::{app_from_crate, arg, App, ArgMatches};
use std::path::Path;

mod bugzilla_query;
mod jira_query;
use crate::jira_query::JiraIssue;

fn build_cli() -> ArgMatches {
    let cli = app_from_crate!()
        .arg(arg!([name] "Optional name to operate on"))
        .arg(
            arg!(
                -c --config <FILE> "Sets a custom config file"
            )
            .required(false)
            // Support non-UTF8 paths
            .allow_invalid_utf8(true),
        )
        .arg(arg!(
            -d --debug ... "Turn debugging information on"
        ))
        .subcommand(
            App::new("jira")
                .about("Query JIRA")
                .arg(arg!(
                    -t --ticket <ID> "The ID of the JIRA issue"
                ))
                .arg(arg!(
                    -a --api_key <KEY> "The JIRA API key"
                ))
                .arg(arg!(
                    -s --server <URL> "The URL to the host with a JIRA instance"
                )),
        )
        .subcommand(
            App::new("bugzilla")
                .about("Query Bugzilla")
                .arg(arg!(
                    -t --ticket <ID> "The ID of the bug"
                ))
                .arg(arg!(
                    -a --api_key <KEY> "The Bugzilla API key"
                ))
                .arg(arg!(
                    -s --server <URL> "The URL to the host with a Bugzilla instance"
                )),
        )
        .get_matches();

    cli
}

fn main() {
    println!("Hello, world!");

    let cli = build_cli();

    if let Some(name) = cli.value_of("name") {
        println!("Value for name: {}", name);
    }

    if let Some(raw_config) = cli.value_of_os("config") {
        let config_path = Path::new(raw_config);
        println!("Value for config: {}", config_path.display());
    }

    match cli.occurrences_of("debug") {
        0 => println!("Debug mode is off"),
        1 => println!("Debug mode is kind of on"),
        2 => println!("Debug mode is on"),
        _ => println!("Don't be crazy"),
    }

    if let Some(cli) = cli.subcommand_matches("jira") {
        let _issue = jira_query::main(
            cli.value_of("server").unwrap(),
            cli.value_of("ticket").unwrap(),
            cli.value_of("api_key").unwrap(),
        );
    }
    if let Some(cli) = cli.subcommand_matches("bugzilla") {
        let _bugs = bugzilla_query::main(
            cli.value_of("server").unwrap(),
            cli.value_of("ticket").unwrap(),
            cli.value_of("api_key").unwrap(),
        );
    }
}
