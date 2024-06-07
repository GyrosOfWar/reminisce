use color_eyre::Result;
use sqlx::SqlitePool;
use time::OffsetDateTime;
use tracing::info;

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
}

#[derive(Debug)]
pub struct NewScreenshot {
    pub path: String,
    pub dpi: f64,
    pub timestamp: OffsetDateTime,
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
        sqlx::query_as!(Screenshot,
            "SELECT rowid AS id, timestamp as \"timestamp: _\", path, dpi, description FROM screenshots WHERE rowid = ?", id
        )

        .fetch_one(&self.pool)
        .await
        .map_err(From::from)
    }

    pub async fn update_description(&self, id: i64, description: &str) -> Result<()> {
        info!("updating screenshot description for id {id} with {description}");
        Ok(())
    }

    pub async fn insert(&self, screenshot: NewScreenshot) -> Result<Screenshot> {
        info!("inserting screenshot {screenshot:?} into database");
        let result = sqlx::query!(
            "INSERT INTO screenshots (timestamp, path, dpi, description)
             VALUES (?, ?, ?, ?) RETURNING rowid",
            screenshot.timestamp,
            screenshot.path,
            screenshot.dpi,
            None::<String>,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(Screenshot {
            id: result.rowid,
            timestamp: screenshot.timestamp,
            path: screenshot.path.clone(),
            dpi: screenshot.dpi,
            description: None,
        })
    }
}
