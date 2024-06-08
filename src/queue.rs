use std::time::Duration;

use age::secrecy::SecretString;
use color_eyre::Result;
use tokio::sync::mpsc;
use tokio::time;
use tracing::{debug, error, info};

use crate::configuration::{Configuration, ProcessingType};
use crate::database::{Database, Screenshot};
use crate::health::SystemHealth;
use crate::image_processing::{llm, ocr};

pub struct WorkItem {
    pub screenshot: Screenshot,
}

pub struct WorkQueue {
    rx: mpsc::UnboundedReceiver<WorkItem>,
    tx: mpsc::UnboundedSender<WorkItem>,
    database: Database,
    system_health: SystemHealth,
    passphrase: SecretString,
    configuration: Configuration,
}

impl WorkQueue {
    pub fn new(database: Database, passphrase: SecretString, configuration: Configuration) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();

        Self {
            rx,
            tx,
            database,
            system_health: SystemHealth::new(),
            passphrase,
            configuration,
        }
    }

    pub fn sender(&self) -> mpsc::UnboundedSender<WorkItem> {
        self.tx.clone()
    }

    async fn is_available_for_work(&self) -> bool {
        let load_below_thresholds = self.system_health.load_below_threshold().await;
        info!("system load below thresholds: {load_below_thresholds}");
        load_below_thresholds
    }

    async fn process_llm(&self, screenshot: &Screenshot) -> Result<()> {
        let result = llm::generate_description(&screenshot, &self.passphrase).await?;
        debug!("llm result: {result}");
        self.database
            .update_description(screenshot.id, &result)
            .await?;

        Ok(())
    }

    async fn process_ocr(&self, screenshot: &Screenshot) -> Result<()> {
        let title = ocr::extract_text(screenshot, &self.passphrase).await?;
        debug!("ocr result: {title}");
        self.database
            .update_text_content(screenshot.id, &title)
            .await?;

        Ok(())
    }

    async fn process_embeddings(&self, screenshot: &Screenshot) -> Result<()> {
        Ok(())
    }

    async fn do_work(&self, WorkItem { screenshot }: WorkItem) -> Result<()> {
        for processing_type in &self.configuration.processing {
            match processing_type {
                ProcessingType::Llm => self.process_llm(&screenshot).await?,
                ProcessingType::Ocr => self.process_ocr(&screenshot).await?,
                ProcessingType::Embeddings => self.process_embeddings(&screenshot).await?,
            }
        }

        Ok(())
    }

    pub async fn start(&mut self) -> Result<()> {
        time::sleep(Duration::from_secs(5)).await;
        info!("starting work queue");

        let pending = self.database.find_pending().await?;
        info!("found {} pending screenshots", pending.len());
        for screenshot in pending {
            let item = WorkItem { screenshot };
            self.tx.send(item)?;
        }

        loop {
            if self.is_available_for_work().await {
                let next_item = self.rx.try_recv();
                if let Ok(item) = next_item {
                    if let Err(e) = self.do_work(item).await {
                        error!("error processing work item: {e:?}");
                    }
                }
            }
            time::sleep(self.configuration.work_interval).await;
        }
    }
}
