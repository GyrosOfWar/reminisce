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
    use std::env;

    color_eyre::install()?;
    dotenvy::dotenv()?;
    let use_tokio_console = env::var("USE_TOKIO_CONSOLE").is_ok();

    if use_tokio_console {
        console_subscriber::init();
    } else {
        tracing_subscriber::fmt().compact().init();
    }

    let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite::memory:".into());
    info!("starting up");
    info!("using databse URL {database_url}");

    let passphrase = encryption::get_passphrase()?;

    let database = Database::new(&database_url).await?;
    let mut work_queue = WorkQueue::new(database);
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
