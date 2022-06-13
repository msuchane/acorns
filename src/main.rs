use color_eyre::eyre::Result;

use cizrna::cli;
use cizrna::run;

fn main() -> Result<()> {
    let cli_arguments = cli::arguments();
    run(&cli_arguments)?;

    Ok(())
}
