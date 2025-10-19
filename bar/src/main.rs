use chrono::Local;
use log::info;

mod battery;
mod freedesktop;
mod iced;
mod sway;
mod system;
mod tray;
mod volume;

const POLL_RATE_MS: u64 = 100;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    setup_logger(log::LevelFilter::Debug)?;

    iced::run()?;

    Ok(())
}

fn setup_logger(log_level: log::LevelFilter) -> Result<(), fern::InitError> {
    // Log to stderr and ~/.local/state/baelyks-notification-server.log
    let log_path = dirs::home_dir()
        .expect("Unable to get the home dir")
        .join(".local/state/")
        .join(env!("CARGO_PKG_NAME"))
        .with_extension("log");

    fern::Dispatch::new()
        .filter(|metadata| {
            metadata
                .target()
                .contains(&env!("CARGO_PKG_NAME").replace("-", "_"))
        })
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{}] [{}] {}",
                record.level(),
                record.target(),
                message
            ))
        })
        .level(log_level)
        .chain(std::io::stderr())
        .chain(
            fern::Dispatch::new()
                .format(|out, message, _| {
                    out.finish(format_args!(
                        "[{}] {}",
                        Local::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                        message
                    ))
                })
                .chain(fern::log_file(&log_path)?),
        )
        .apply()?;

    info!(
        "Starting {} v{} with log level: {}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        log_level
    );

    Ok(())
}
