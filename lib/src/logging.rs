pub struct Logger {
    name: &'static str,
    version: &'static str,
    level: log::LevelFilter,
    only_baelyks: bool,
}

impl Logger {
    pub fn with_name(name: &'static str) -> Self {
        Self {
            name,
            ..Default::default()
        }
    }

    pub fn name(mut self, name: &'static str) -> Self {
        self.name = name;
        self
    }

    pub fn version(mut self, version: &'static str) -> Self {
        self.version = version;
        self
    }

    pub fn level(mut self, level: log::LevelFilter) -> Self {
        self.level = level;
        self
    }

    pub fn only_baelyks(mut self, only_baelyks: bool) -> Self {
        self.only_baelyks = only_baelyks;
        self
    }

    pub fn setup(self) -> Result<(), fern::InitError> {
        fern::Dispatch::new()
            .filter(move |metadata| !self.only_baelyks || metadata.target().contains("baelyks"))
            .format(|out, message, record| {
                out.finish(format_args!(
                    "[{}] [{}] {}",
                    record.level(),
                    record.target(),
                    message
                ))
            })
            .level(self.level)
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
            "Starting {} v{} with log level: {} (dep logs {})",
            self.name,
            self.version,
            self.level,
            if self.only_baelyks { "off" } else { "on" }
        );

        Ok(())
    }
}

impl Default for Logger {
    fn default() -> Self {
        Self {
            name: env!("CARGO_PKG_NAME"),
            version: env!("CARGO_PKG_VERSION"),
            level: log::LevelFilter::Info,
            only_baelyks: true,
        }
    }
}
