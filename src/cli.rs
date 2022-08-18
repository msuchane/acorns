/*
cizrna: Generate an AsciiDoc release notes document from tracking tickets.
Copyright (C) 2022  Marek Suchánek  <msuchane@redhat.com>

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

use clap::{arg, command, ArgMatches, Command};

/// Define the command-line arguments of the tool.
#[must_use]
pub fn arguments() -> ArgMatches {
    let app = command!()
        .arg(arg!(
            -v --verbose ... "Display more detailed progress messages."
        ).global(true))
        .subcommand(
            Command::new("build")
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
                    -s --service <name> "The type of the issue tracker service."
                ).possible_values(["bugzilla", "jira"]))
        // Require using at least one subcommand or some other argument.
        ).arg_required_else_help(true);

    app.get_matches()
}
