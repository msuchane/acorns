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

use clap::{Parser, Subcommand};

/// Define the command-line arguments of the tool.
#[must_use]
pub fn arguments() -> Cli {
    Cli::parse()
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Display more detailed progress messages.
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Build release notes from a configuration directory.
    Build {
        /// Path to the configuration directory. The default is the current working directory.
        #[arg(value_parser, value_name = "DIR", default_value = ".")]
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
    Ticket {
        /// The type of the issue tracker service.
        #[arg(value_name = "SERVICE")]
        //tracker: crate::config::tracker::Service,
        tracker: String,
        /// The ID of the ticket.
        #[arg(value_name = "ID")]
        id: String,
        /// The trackers configuration file.
        #[arg(
            short,
            long,
            value_parser,
            value_name = "FILE",
            default_value = "./cizrna/trackers.yaml"
        )]
        config: PathBuf,
        /// The API key to access the tracker.
        #[arg(short, long, value_name = "FILE")]
        api_key: Option<String>,
    },
    /// Convert a CoRN 3 configuration file to the new format.
    Convert {
        /// The legacy corn.yaml configuration file.
        #[arg(
            short,
            long,
            value_parser,
            value_name = "FILE",
            default_value = "./corn.yaml"
        )]
        legacy_config: PathBuf,
        /// The legacy corn.yaml configuration file.
        #[arg(
            short,
            long,
            value_parser,
            value_name = "FILE",
            default_value = "./tickets.yaml"
        )]
        new_config: PathBuf,
    },
}
