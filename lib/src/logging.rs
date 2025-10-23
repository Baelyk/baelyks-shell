pub fn setup_logger(log_level: log::LevelFilter) -> Result<(), fern::InitError> {
    fern::Dispatch::new()
        .filter(move |metadata| {
            log_level == log::LevelFilter::Trace || metadata.target().contains("baelyks")
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
        .chain(fern::Dispatch::new().format(|out, message, _| {
            out.finish(format_args!(
                "[{}] {}",
                chrono::Local::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                message
            ))
        }))
        .apply()?;

    log::info!(
        "Starting {} v{} with log level: {}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        log_level
    );

    Ok(())
}
