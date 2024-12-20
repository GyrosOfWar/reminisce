use std::fs::File;
use std::time::Duration;

use camino::Utf8PathBuf;
use color_eyre::eyre::Context;
use color_eyre::Result;
use serde::Deserialize;
use serde_with::{serde_as, DurationSeconds};

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum ProcessingType {
    Llm,
    Ocr,
    Embeddings,
}

#[serde_as]
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Configuration {
    /// How often the app takes a screenshot.
    #[serde_as(as = "DurationSeconds<u64>")]
    pub screenshot_interval: Duration,

    /// How long the app waits between processing screenshots.
    #[serde_as(as = "DurationSeconds<u64>")]
    pub work_interval: Duration,

    /// The directory where screenshots are stored.
    pub screenshot_directory: Utf8PathBuf,

    /// The SQLite database file name.
    pub database_file_name: String,

    /// How to process the screenshots.
    pub processing: Vec<ProcessingType>,

    pub similarity_threshold: f32,
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            screenshot_interval: Duration::from_secs(60),
            work_interval: Duration::from_secs(120),
            screenshot_directory: Utf8PathBuf::from("screenshots"),
            database_file_name: "reminisce.sqlite3".to_string(),
            processing: vec![ProcessingType::Ocr, ProcessingType::Embeddings],
            similarity_threshold: 0.9,
        }
    }
}

pub fn load() -> Result<Configuration> {
    let file = File::open("reminisce.json").wrap_err("unable to open reminisce.json")?;
    let config = serde_json::from_reader(file).wrap_err("unable to deserialize configuration")?;
    Ok(config)
}
