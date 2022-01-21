use clap::{app_from_crate, arg, App, ArgMatches};

pub fn build_cli() -> ArgMatches {
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
