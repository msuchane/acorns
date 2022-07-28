// Enable additional clippy lints by default.
#![warn(
    clippy::pedantic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::clone_on_ref_ptr,
    clippy::todo
)]
// Disable the documentation clippy lint, so that it stops suggesting backticks around AsciiDoc.
#![allow(clippy::doc_markdown)]

use color_eyre::eyre::Result;

use cizrna::cli;
use cizrna::run;

fn main() -> Result<()> {
    // Enable full-featured error logging.
    color_eyre::install()?;

    // Collect command-line arguments.
    let cli_arguments = cli::arguments();

    // Run the main program.
    run(&cli_arguments)?;

    Ok(())
}
