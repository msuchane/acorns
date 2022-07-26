use color_eyre::eyre::{Result, WrapErr};
use simplelog::{ColorChoice, Config, LevelFilter, TermLogger, TerminalMode};

/// This function initializes the `simplelog` logging system, which plugs into the `log`
/// infrastructure. The function returns nothing. It only affects the global state when it runs.
pub fn initialize_logger(verbose: u64) -> Result<()> {
    // Set the verbosity level based on the command-line options.
    // The `verbose` option captures the number of occurrences of `--verbose`.
    let verbosity = match verbose {
        // By default, display some essential progress.
        0 => LevelFilter::Info,
        1 => LevelFilter::Debug,
        _ => LevelFilter::Trace,
    };

    //let config = ConfigBuilder::new()
    // Display a time stamp only for the most verbose level.
    //.set_time_level(LevelFilter::Trace)
    // Display the thread number only for the most verbose level.
    //.set_thread_level(LevelFilter::Trace)
    //.build();

    TermLogger::init(
        verbosity,
        Config::default(),
        // Mixed mode prints errors to stderr and info to stdout. Not sure about the other levels.
        TerminalMode::default(),
        // Try to use color if possible.
        ColorChoice::Auto,
    )
    .wrap_err("Failed to configure the terminal logging.")?;

    Ok(())
}
