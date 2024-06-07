use color_eyre::Result;
use database::Database;
use queue::WorkQueue;
use recorder::ScreenRecorder;
use std::time::Duration;
use tracing::info;

mod database;
mod encryption;
mod health;
mod llm;
mod queue;
mod recorder;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    dotenvy::dotenv()?;

    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite::memory:".into());
    tracing_subscriber::fmt().compact().init();
    info!("starting up");

    let passphrase = encryption::get_passphrase()?;

    let databse = Database::new(&database_url).await?;
    let mut work_queue = WorkQueue::new(databse);
    let sender = work_queue.sender();
    let screen_recorder = ScreenRecorder::new(Duration::from_secs(30), sender, passphrase).await?;

    tokio::spawn(async move {
        screen_recorder
            .start()
            .await
            .expect("failed to start screen recording");
    });

    work_queue.start().await?;

    Ok(())
}
