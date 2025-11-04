mod battery;
mod iced;
mod sway;
mod system;
mod tray;
mod volume;

const POLL_RATE_MS: u64 = 100;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    baelyks_shell_lib::logging::Logger::with_name(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .level(log::LevelFilter::Debug)
        .setup()?;

    iced::run()?;

    Ok(())
}
