use clap::{app_from_crate, arg, App, ArgMatches};
use std::path::Path;

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
            -i --issue <ID> "The ID of the JIRA issue"
        ))
        .arg(arg!(
            -a --api_key <KEY> "The JIRA API key"
        ))
        .arg(arg!(
            -h --jira_host <URL> "The URL to the host with a JIRA instance"
        ))
        .arg(arg!(
            -d --debug ... "Turn debugging information on"
        ))
        .subcommand(
            App::new("test")
                .about("does testing things")
                .arg(arg!(-l --list "lists test values")),
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

    if let Some(cli) = cli.subcommand_matches("test") {
        // "$ myapp test" was run
        if cli.is_present("list") {
            // "$ myapp test -l" was run
            println!("Printing testing lists...");
        } else {
            println!("Not printing testing lists...");
        }
    }

    jira_query::main(
        cli.value_of("jira_host").unwrap(),
        cli.value_of("issue").unwrap(),
        cli.value_of("api_key").unwrap(),
    );
}
