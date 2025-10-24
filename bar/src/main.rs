mod battery;
mod iced;
mod sway;
mod system;
mod tray;
mod volume;

const POLL_RATE_MS: u64 = 100;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    baelyks_shell_lib::logging::setup_logger(log::LevelFilter::Debug)?;

    iced::run()?;

    Ok(())
}
