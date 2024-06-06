use std::{thread, time::Duration};

use color_eyre::Result;
use tracing::error;

use crate::{database::Database, llm};
use tokio::sync::mpsc;

const WORK_INTERVAL: Duration = Duration::from_secs(60);

pub struct WorkItem {
    pub screenshot_id: i64,
}

pub struct WorkQueue {
    rx: mpsc::UnboundedReceiver<WorkItem>,
    tx: mpsc::UnboundedSender<WorkItem>,
    database: Database,
}

impl WorkQueue {
    pub fn new(database: Database) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();

        Self { rx, tx, database }
    }

    pub fn sender(&self) -> mpsc::UnboundedSender<WorkItem> {
        self.tx.clone()
    }

    fn is_available_for_work(&self) -> bool {
        // TODO: check if system load is low
        true
    }

    fn do_work(&self, item: WorkItem) -> Result<()> {
        let screenshot = self.database.find_by_id(item.screenshot_id)?;
        // TODO pre-process the screenshot
        let description = llm::generate_description(&screenshot)?;
        // TODO post-process the description if necessary
        self.database
            .update_description(screenshot.id, &description)?;

        Ok(())
    }

    pub async fn start(&mut self) -> Result<()> {
        loop {
            if self.is_available_for_work() {
                let next_item = self.rx.try_recv();
                if let Ok(item) = next_item {
                    if let Err(e) = self.do_work(item) {
                        error!("error processing work item: {e:?}");
                    }
                }
            }

            thread::sleep(WORK_INTERVAL);
        }
    }
}
