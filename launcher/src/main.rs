use clap::Parser;
use log::debug;

mod iced;
mod providers;
mod searcher;
mod selectable_rows;

/// A notification server using Eww to display notifications
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Log level: can be Off, Error, Warn, Info, Debug, or Trace
    #[arg(long, default_value_t = log::LevelFilter::Info)]
    log: log::LevelFilter,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    baelyks_shell_lib::logging::Logger::with_name(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .level(args.log)
        .setup()?;

    debug!("Command line arguments: {:#?}", args);

    crate::iced::run()?;
    Ok(())
}
