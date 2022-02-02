use clap::{app_from_crate, arg, App, AppSettings, ArgMatches};

pub fn arguments() -> ArgMatches {
    let app = app_from_crate!()
        .arg(arg!(
            -v --verbose ... "Display more detailed progress messages."
        ).global(true))
        .subcommand(
            App::new("build")
                .about("Build release notes from a configuration directory.")
                .arg(arg!([project] "Path to the configuration directory. The default is the current working directory.").allow_invalid_utf8(true))
                .arg(
                    arg!(
                        -t --tickets [FILE] "A configuration file containing tickets."
                    )
                    // Support non-UTF8 paths
                    .allow_invalid_utf8(true),
                )
                .arg(
                    arg!(
                        -T --trackers [FILE] "A configuration file containing trackers."
                    )
                    // Support non-UTF8 paths
                    .allow_invalid_utf8(true),
                )
        )
        .subcommand(
            App::new("ticket")
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
                ).possible_values(["bugzilla", "jira"]))
        // Require using at least one subcommand or some other argument.
        ).setting(AppSettings::ArgRequiredElseHelp);

    app.get_matches()
}
