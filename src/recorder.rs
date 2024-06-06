use age::secrecy::SecretString;
use color_eyre::{eyre::OptionExt, Result};
use crabgrab::{
    capturable_content::{CapturableContent, CapturableContentFilter},
    capture_stream::{CaptureAccessToken, CaptureConfig, CaptureStream},
    feature::screenshot,
    prelude::VideoFrameBitmap,
};
use std::{
    sync::atomic::{AtomicI64, Ordering},
    time::Duration,
};
use time::OffsetDateTime;
use tokio::sync::mpsc;
use tracing::info;

use crate::{database::Screenshot, queue::WorkItem};

static SCREENSHOT_ID: AtomicI64 = AtomicI64::new(0);

pub struct ScreenRecorder {
    interval: Duration,
    sender: mpsc::UnboundedSender<WorkItem>,
    passphrase: SecretString,
    access_token: CaptureAccessToken,
}

impl ScreenRecorder {
    pub async fn new(
        interval: Duration,
        sender: mpsc::UnboundedSender<WorkItem>,
        passphrase: SecretString,
    ) -> Result<Self> {
        let access_token = CaptureStream::test_access(false);
        let access_token = match access_token {
            Some(t) => t,
            None => CaptureStream::request_access(false)
                .await
                .ok_or_eyre("unable to get capture permission")?,
        };

        Ok(Self {
            interval,
            sender,
            passphrase,
            access_token,
        })
    }

    async fn create_screenshot(&self) -> Result<Screenshot> {
        let filter = CapturableContentFilter::NORMAL_WINDOWS;
        let content = CapturableContent::new(filter).await?;
        let formats = CaptureStream::supported_pixel_formats();
        let display = content
            .displays()
            .next()
            .ok_or_eyre("must have at least one display")?;
        let config = CaptureConfig::with_display(display, formats[0]);

        let video_frame = screenshot::take_screenshot(self.access_token.clone(), config).await?;
        let bitmap = video_frame.get_bitmap()?;

        let screenshot = Screenshot {
            id: SCREENSHOT_ID.fetch_add(1, Ordering::SeqCst),
            description: None,
            path: "todo".into(),
            timestamp: OffsetDateTime::now_utc(),
        };

        Ok(screenshot)
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
