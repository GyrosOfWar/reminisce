use std::time::Duration;

use camino::Utf8PathBuf;
use color_eyre::Result;
use config::Config;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Configuration {
    /// How often the app takes a screenshot.
    pub screenshot_interval: Duration,
    /// How long the app waits between processing screenshots.
    pub work_interval: Duration,
    /// The directory where screenshots are stored.
    pub screenshot_directory: Utf8PathBuf,
    /// The SQLite database URL.
    pub database_url: String,
}

pub fn load() -> Result<Configuration> {
    let settings = Config::builder()
        .add_source(config::File::with_name("reminisce"))
        .add_source(config::Environment::with_prefix("REMINISCE_"))
        .build()?;

    settings.try_deserialize().map_err(From::from)
}
