use age::secrecy::SecretString;
use color_eyre::Result;
use crabgrab::feature::screenshot;
use std::{
    sync::atomic::{AtomicI64, Ordering},
    time::Duration,
};
use tokio::sync::mpsc;
use tracing::info;

use crate::{database::Screenshot, queue::WorkItem};

static SCREENSHOT_ID: AtomicI64 = AtomicI64::new(0);

pub struct ScreenRecorder {
    interval: Duration,
    sender: mpsc::UnboundedSender<WorkItem>,
    passphrase: SecretString,
}

impl ScreenRecorder {
    pub fn new(
        interval: Duration,
        sender: mpsc::UnboundedSender<WorkItem>,
        passphrase: SecretString,
    ) -> Self {
        Self {
            interval,
            sender,
            passphrase,
        }
    }

    async fn create_screenshot(&self) -> Result<Screenshot> {
        let video_frame = screenshot::take_screenshot(token, config).await?;
        todo!()
    }

    pub async fn start(&self) -> Result<()> {
        loop {
            let screenshot = self.create_screenshot().await?;
            self.sender.send(WorkItem {
                screenshot_id: screenshot.id,
            })?;
            tokio::time::sleep(self.interval).await;
        }
    }
}
