use clap::{app_from_crate, arg, App, ArgMatches};

pub fn arguments() -> ArgMatches {
    let cli = app_from_crate!()
        .arg(
            arg!(
                -t --tickets <FILE> "A configuration file containing tickets."
            )
            .required(true)
            // Support non-UTF8 paths
            .allow_invalid_utf8(true),
        )
        .arg(
            arg!(
                -T --trackers <FILE> "A configuration file containing trackers."
            )
            .required(true)
            // Support non-UTF8 paths
            .allow_invalid_utf8(true),
        )
        .arg(arg!(
            -v --verbose ... "Display more detailed progress messages."
        ))
        .subcommand(
            App::new("jira")
                .about("Query Jira")
                .arg(arg!(
                    -t --ticket <ID> "The ID of the Jira issue"
                ))
                .arg(arg!(
                    -a --api_key <KEY> "The Jira API key"
                ))
                .arg(arg!(
                    -s --server <URL> "The URL to the host with a Jira instance"
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
