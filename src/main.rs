/*
acorns: Generate an AsciiDoc release notes document from tracking tickets.
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

use color_eyre::eyre::Result;

use acorns::cli;
use acorns::run;

fn main() -> Result<()> {
    // Enable full-featured error logging.
    color_eyre::install()?;

    // Collect command-line arguments.
    let cli_arguments = cli::arguments();

    // Run the main program.
    run(&cli_arguments)?;

    Ok(())
}
