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

use std::path::PathBuf;

use bpaf::Bpaf;

/// Define the command-line arguments of the tool.
#[must_use]
pub fn arguments() -> Cli {
    let usage_prefix = "Usage: cizrna {usage}";
    cli().usage(usage_prefix).run()
}

#[derive(Clone, Debug, Bpaf)]
#[bpaf(options, version)]
/// Generate an AsciiDoc release notes document from tracking tickets.
pub struct Cli {
    /// Display more detailed progress messages.
    #[bpaf(short, long, switch, many, map(vec_len))]
    pub verbose: usize,

    #[bpaf(external(commands))]
    pub command: Commands,
}

#[derive(Clone, Debug, Bpaf)]
pub enum Commands {
    /// Build release notes from a configuration directory.
    #[bpaf(command)]
    Build {
        /// Path to the configuration directory. The default is the current working directory.
        #[bpaf(positional::<PathBuf>("DIR"), fallback(".".into()))]
        project: PathBuf,
        // Disabling the optional config paths for now.
        // It's questionable if it's even useful to specify these separately.
        /*
        /// A configuration file containing tickets.
        #[clap(short, long, value_name = "FILE")]
        tickets: Option<PathBuf>,
        /// A configuration file containing trackers.
        #[clap(short='T', long, value_name = "FILE")]
        trackers: Option<PathBuf>,
        /// A configuration file containing templates.
        #[clap(short='e', long, value_name = "FILE")]
        templates: Option<PathBuf>,
        */
    },
    /// Query a single ticket.
    #[bpaf(command)]
    Ticket {
        /// The trackers configuration file.
        #[bpaf(
            short,
            long,
            argument("FILE"),
            fallback("./cizrna/trackers.yaml".into())
        )]
        config: PathBuf,
        /// The API key to access the tracker.
        #[bpaf(short, long, argument("SECRET"))]
        api_key: Option<String>,
        /// The type of the issue tracker service.
        #[bpaf(positional::<String>("SERVICE"))]
        tracker: String,
        /// The ID of the ticket.
        #[bpaf(positional::<String>("ID"))]
        id: String,
    },
    /// Convert a CoRN 3 configuration file to the new format.
    #[bpaf(command)]
    Convert {
        /// The legacy corn.yaml configuration file.
        #[bpaf(
            short,
            long,
            argument("FILE"),
            fallback("./corn.yaml".into())
        )]
        legacy_config: PathBuf,
        /// The new, converted configuration file.
        #[bpaf(
            short,
            long,
            argument("FILE"),
            fallback("./tickets.yaml".into())
        )]
        new_config: PathBuf,
    },
    /// Create a sample release notes project with basic configuration.
    #[bpaf(command)]
    Init {
        /// Path to the project directory. The default is the current working directory.
        #[bpaf(
            positional::<PathBuf>("DIR"),
            fallback(".".into())
        )]
        directory: PathBuf,
    },
}

/// Calculate the length of a vector for repeating flags, such as verbosity.
///
/// This function has to take the argument by value because that's how
/// the `bpaf` parser passes it in the map application.
#[allow(clippy::needless_pass_by_value)]
fn vec_len<T>(vec: Vec<T>) -> usize {
    vec.len()
}
