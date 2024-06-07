use std::time::Duration;

use age::secrecy::SecretString;
use color_eyre::Result;
use tokio::time;
use tracing::{error, info};

use crate::{
    database::{Database, Screenshot},
    health::SystemHealth,
    image_processing::llm,
};
use tokio::sync::mpsc;

const WORK_INTERVAL: Duration = Duration::from_secs(15);

pub struct WorkItem {
    pub screenshot: Screenshot,
}

pub struct WorkQueue {
    rx: mpsc::UnboundedReceiver<WorkItem>,
    tx: mpsc::UnboundedSender<WorkItem>,
    database: Database,
    system_health: SystemHealth,
    passphrase: SecretString,
}

impl WorkQueue {
    pub fn new(database: Database, passphrase: SecretString) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();

        Self {
            rx,
            tx,
            database,
            system_health: SystemHealth::new(),
            passphrase,
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

    async fn do_work(&self, item: WorkItem) -> Result<()> {
        let screenshot = item.screenshot;
        // TODO pre-process the screenshot
        let description = llm::generate_description(&screenshot, &self.passphrase).await?;
        // TODO post-process the description if necessary
        self.database
            .update_description(screenshot.id, &description)
            .await?;

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
            time::sleep(WORK_INTERVAL).await;
        }
    }
}
