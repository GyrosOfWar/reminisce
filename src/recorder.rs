use age::secrecy::SecretString;
use color_eyre::{eyre::OptionExt, Result};
use crabgrab::{
    capturable_content::{CapturableContent, CapturableContentFilter},
    capture_stream::{CaptureAccessToken, CaptureConfig, CapturePixelFormat, CaptureStream},
    feature::screenshot,
    prelude::{FrameBitmap, VideoFrameBitmap},
};
use image::{DynamicImage, ImageBuffer, ImageFormat, RgbaImage};
use std::{io::Cursor, time::Duration};
use time::OffsetDateTime;
use tokio::sync::mpsc;
use tracing::info;

use crate::{
    database::{Database, NewScreenshot, Screenshot},
    encryption::encrypt_file,
    queue::WorkItem,
};

pub struct ScreenRecorder {
    interval: Duration,
    sender: mpsc::UnboundedSender<WorkItem>,
    passphrase: SecretString,
    access_token: CaptureAccessToken,
    database: Database,
}

impl ScreenRecorder {
    pub async fn new(
        database: Database,
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
            database,
            interval,
            sender,
            passphrase,
            access_token,
        })
    }

    async fn create_screenshot(&self) -> Result<Screenshot> {
        let filter = CapturableContentFilter::NORMAL_WINDOWS;
        let content = CapturableContent::new(filter).await?;
        // supported by both windows and macos
        let format = CapturePixelFormat::Bgra8888;
        let display = content
            .displays()
            .next()
            .ok_or_eyre("must have at least one display")?;
        let config = CaptureConfig::with_display(display, format);

        let video_frame = screenshot::take_screenshot(self.access_token.clone(), config).await?;
        let bitmap = video_frame.get_bitmap()?;

        let pixels = match bitmap {
            FrameBitmap::BgraUnorm8x4(bitmap) => bitmap,
            FrameBitmap::RgbaUnormPacked1010102(_) => unreachable!(),
            FrameBitmap::RgbaF16x4(_) => unreachable!(),
            FrameBitmap::YCbCr(_) => unreachable!(),
        };

        let data: Vec<_> = pixels
            .data
            .into_iter()
            .copied()
            .flat_map(|[b, g, r, a]| [r, g, b, a])
            .collect();

        let image: RgbaImage =
            ImageBuffer::from_raw(pixels.width as u32, pixels.height as u32, data)
                .ok_or_eyre("unable to create image buffer")?;
        let image = DynamicImage::from(image);
        let image = image.to_rgb8();

        let timestamp = OffsetDateTime::now_utc().unix_timestamp();
        let path = format!("screenshots/{}.jpeg.enc", timestamp);
        let mut bytes = Cursor::new(vec![]);
        image.write_to(&mut bytes, ImageFormat::Jpeg)?;
        encrypt_file(&path, self.passphrase.clone(), bytes.get_ref())?;
        let screenshot = NewScreenshot {
            path,
            // video_frame.dpi() crashes on macos
            dpi: 72.0,
            timestamp: OffsetDateTime::now_utc(),
        };

        let screenshot = self.database.insert(screenshot).await?;

        Ok(screenshot)
    }

    pub async fn start(&self) -> Result<()> {
        info!("starting screen recorder");
        loop {
            let screenshot = self.create_screenshot().await?;
            self.sender.send(WorkItem { screenshot })?;
            tokio::time::sleep(self.interval).await;
        }
    }
}
