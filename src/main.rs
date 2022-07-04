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
