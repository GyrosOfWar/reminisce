use age::secrecy::SecretString;
use color_eyre::Result;
use image::{ImageFormat, RgbImage};
use sqlx::SqlitePool;
use time::OffsetDateTime;
use tracing::info;

use crate::encryption;

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
pub enum ProcessingStatus {
    Pending,
    Finished,
}

pub struct Screenshot {
    pub id: i64,
    /// When the screenshot was taken
    pub timestamp: OffsetDateTime,
    /// Path to the encrypted screenshot file on disk
    pub path: String,

    /// The DPI of the screenshot (useful for retina screens etc.)
    pub dpi: f64,

    /// LLM-generated description of the screenshot.
    pub description: Option<String>,

    /// OCR text extracted from the screenshot.
    pub text_content: Option<String>,

    pub status: ProcessingStatus,

    pub window_title: String,

    pub application_name: String,
    // TODO embeddings
}

impl Screenshot {
    pub fn load_image_bytes(&self, passphrase: &SecretString) -> Result<Vec<u8>> {
        let bytes = encryption::decrypt_file(&self.path, passphrase)?;
        Ok(bytes)
    }

    pub fn load_image(&self, passphrase: &SecretString) -> Result<RgbImage> {
        let bytes = self.load_image_bytes(passphrase)?;
        let image = image::load_from_memory_with_format(&bytes, ImageFormat::Jpeg)?;
        Ok(image.into_rgb8())
    }
}

#[derive(Debug)]
pub struct NewScreenshot {
    pub path: String,
    pub dpi: f64,
    pub timestamp: OffsetDateTime,
    pub window_title: String,
    pub application_name: String,
}

#[derive(Clone, Debug)]
pub struct Database {
    pool: SqlitePool,
}

impl Database {
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = SqlitePool::connect(database_url).await?;

        Ok(Self { pool })
    }

    pub async fn find_by_id(&self, id: i64) -> Result<Screenshot> {
        sqlx::query_as!(
            Screenshot,
            "SELECT rowid AS id, timestamp AS \"timestamp: _\", path, dpi, description, status AS \"status: _\", window_title, application_name, text_content
            FROM screenshots 
            WHERE rowid = ?",
            id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(From::from)
    }

    pub async fn find_most_recent_screenshot(&self) -> Result<Option<Screenshot>> {
        sqlx::query_as!(
            Screenshot,
            "SELECT rowid AS id, timestamp AS \"timestamp: _\", path, dpi, description, status AS \"status: _\", window_title, application_name, text_content
            FROM screenshots 
            ORDER BY timestamp DESC
            LIMIT 1"
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(From::from)
    }

    pub async fn update_description(&self, id: i64, description: &str) -> Result<()> {
        sqlx::query!(
            "UPDATE screenshots SET description = ?, status = ? WHERE rowid = ?",
            description,
            ProcessingStatus::Finished,
            id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn update_text_content(&self, id: i64, text_content: &str) -> Result<()> {
        sqlx::query!(
            "UPDATE screenshots SET text_content = ?, status = ? WHERE rowid = ?",
            text_content,
            ProcessingStatus::Finished,
            id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn insert(&self, screenshot: NewScreenshot) -> Result<Screenshot> {
        info!("inserting screenshot {screenshot:?} into database");
        let result = sqlx::query!(
            "INSERT INTO screenshots (timestamp, path, dpi, description, status, window_title, application_name)
             VALUES (?, ?, ?, ?, ?, ?, ?) RETURNING rowid",
            screenshot.timestamp,
            screenshot.path,
            screenshot.dpi,
            None::<String>,
            ProcessingStatus::Pending,
            screenshot.window_title,
            screenshot.application_name
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(Screenshot {
            id: result.rowid,
            timestamp: screenshot.timestamp,
            path: screenshot.path.clone(),
            dpi: screenshot.dpi,
            description: None,
            status: ProcessingStatus::Pending,
            window_title: screenshot.window_title,
            application_name: screenshot.application_name,
            text_content: None,
        })
    }

    pub async fn find_all(&self) -> Result<Vec<Screenshot>> {
        sqlx::query_as!(
            Screenshot,
            "SELECT rowid AS id, timestamp AS \"timestamp: _\", path, dpi, description, status AS \"status: _\", window_title, application_name, text_content
            FROM screenshots"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(From::from)
    }

    pub async fn find_pending(&self) -> Result<Vec<Screenshot>> {
        sqlx::query_as!(
            Screenshot,
            "SELECT rowid AS id, timestamp AS \"timestamp: _\", path, dpi, description, status AS \"status: _\", window_title, application_name, text_content
            FROM screenshots 
            WHERE status = ?",
            ProcessingStatus::Pending
        )
        .fetch_all(&self.pool)
        .await
        .map_err(From::from)
    }

    pub async fn delete_all(&self) -> Result<()> {
        sqlx::query!("DELETE FROM screenshots")
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
