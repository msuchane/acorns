use clap::builder::{PossibleValuesParser, ValueParser};
use clap::{arg, command, ArgMatches, Command};

/// Define the command-line arguments of the tool.
pub fn arguments() -> ArgMatches {
    let app = command!()
        .arg(arg!(
            -v --verbose ... "Display more detailed progress messages."
        ).global(true))
        .subcommand(
            Command::new("build")
                .about("Build release notes from a configuration directory.")
                .arg(arg!([project] "Path to the configuration directory. The default is the current working directory.").value_parser(ValueParser::os_string()))
                .arg(
                    arg!(
                        -t --tickets [FILE] "A configuration file containing tickets."
                    )
                    // Support non-UTF8 paths
                    .value_parser(ValueParser::os_string()),
                )
                .arg(
                    arg!(
                        -T --trackers [FILE] "A configuration file containing trackers."
                    )
                    // Support non-UTF8 paths
                    .value_parser(ValueParser::os_string()),
                )
        )
        .subcommand(
            Command::new("ticket")
                .about("Query a single ticket.")
                .arg(arg!(
                    -i --id <ID> "The ID of the ticket"
                ))
                .arg(arg!(
                    -a --api_key <KEY> "The Bugzilla API key"
                ))
                .arg(arg!(
                    -H --host <URL> "The URL to the host with a Bugzilla instance"
                ))
                .arg(arg!(
                    -s --service <URL> "The URL to the host with a Bugzilla instance"
                ).value_parser(PossibleValuesParser::new(["bugzilla", "jira"])))
        // Require using at least one subcommand or some other argument.
        ).arg_required_else_help(true);

    app.get_matches()
}
