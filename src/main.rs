use color_eyre::eyre::Result;

use cizrna::run;
use cizrna::cli;

fn main() -> Result<()> {
    let cli_arguments = cli::arguments();
    run(&cli_arguments)?;

    Ok(())
}
