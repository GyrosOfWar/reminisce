use age::secrecy::SecretString;
use camino::Utf8PathBuf;
use color_eyre::Result;
use configuration::Configuration;
use database::Database;
use queue::WorkQueue;
use recorder::ScreenRecorder;
use tracing::info;

mod configuration;
mod database;
mod encryption;
mod health;
mod image_processing;
mod queue;
mod recorder;

async fn start_recorder(
    database: Database,
    passphrase: SecretString,
    configuration: Configuration,
) -> Result<()> {
    let mut work_queue =
        WorkQueue::new(database.clone(), passphrase.clone(), configuration.clone());
    let sender = work_queue.sender();
    let screen_recorder = ScreenRecorder::new(
        database,
        configuration.screenshot_interval,
        sender,
        passphrase,
        configuration,
    )
    .await?;

    tokio::spawn(async move {
        screen_recorder
            .start()
            .await
            .expect("failed to start screen recording");
    });

    work_queue.start().await?;

    Ok(())
}

async fn decrypt_screenshots(
    database: Database,
    passphrase: SecretString,
    configuration: Configuration,
) -> Result<()> {
    let screenshots = database.find_all().await?;
    for screenshot in screenshots {
        let bytes = encryption::decrypt_file(&screenshot.path, &passphrase)?;
        let path = configuration
            .screenshot_directory
            .join(format!("{}.jpeg", screenshot.id));
        tokio::fs::write(path, bytes).await?;
    }

    Ok(())
}

async fn delete_everything(database: Database, configuration: Configuration) -> Result<()> {
    use std::fs;

    database.delete_all().await?;
    let files = fs::read_dir(configuration.screenshot_directory)?;
    for file in files {
        let file = file?;
        let path = Utf8PathBuf::try_from(file.path())?;
        let should_delete = path
            .file_name()
            .map(|f| f.ends_with(".enc"))
            .unwrap_or(false)
            && file.file_type()?.is_file();

        if should_delete {
            fs::remove_file(path)?;
        }
    }

    Ok(())
}

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

    let configuration = configuration::load()?;
    // let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite::memory:".into());
    info!("starting up");
    info!("using configuration {configuration:?}");

    let passphrase = encryption::get_passphrase()?;
    let database = Database::new(&configuration.database_url).await?;

    let argument = env::args().nth(1);
    match argument.as_deref() {
        Some("record") => start_recorder(database, passphrase, configuration).await?,
        Some("decrypt") => decrypt_screenshots(database, passphrase, configuration).await?,
        Some("delete") => delete_everything(database, configuration).await?,
        _ => {
            start_recorder(database, passphrase, configuration).await?;
        }
    }

    Ok(())
}
