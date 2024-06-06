use color_eyre::Result;
use tracing::info;

pub struct Screenshot {
    pub id: i64,
    pub timestamp: time::OffsetDateTime,
    pub path: String,

    /// LLM-generated description of the screenshot.
    pub description: Option<String>,
}

pub struct Database {}

impl Database {
    pub fn find_by_id(&self, id: i64) -> Result<Screenshot> {
        info!("returning mock screenshot");
        Ok(Screenshot {
            id,
            timestamp: time::OffsetDateTime::now_utc(),
            path: format!("/path/to/screenshot/{}.png", id),
            description: None,
        })
    }

    pub fn update_description(&self, id: i64, description: &str) -> Result<()> {
        info!("updating screenshot description for id {id} with {description}");
        Ok(())
    }
}
