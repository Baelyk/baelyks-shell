mod iced;
mod providers;
mod searcher;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    baelyks_shell_lib::logging::setup_logger(log::LevelFilter::Debug)?;

    crate::iced::run()?;
    Ok(())
}
