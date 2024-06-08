use age::secrecy::SecretString;
use color_eyre::{
    eyre::{eyre, OptionExt},
    Result,
};
use crabgrab::{
    capturable_content::{CapturableContent, CapturableContentFilter},
    capture_stream::{CaptureAccessToken, CaptureConfig, CapturePixelFormat, CaptureStream},
    feature::screenshot,
    prelude::{FrameBitmap, VideoFrameBitmap},
};
use image::{DynamicImage, ImageBuffer, ImageFormat, RgbImage, RgbaImage};
use std::{io::Cursor, time::Duration};
use time::OffsetDateTime;
use tokio::sync::mpsc;
use tracing::info;

use crate::{
    configuration::Configuration,
    database::{Database, NewScreenshot, Screenshot},
    encryption::encrypt_file,
    image_processing::similarity::is_similar,
    queue::WorkItem,
};

struct CapturedScreenshot {
    bitmap: FrameBitmap,
    app_name: String,
    title: String,
}

pub struct ScreenRecorder {
    interval: Duration,
    sender: mpsc::UnboundedSender<WorkItem>,
    passphrase: SecretString,
    access_token: CaptureAccessToken,
    database: Database,
    configuration: Configuration,
}

impl ScreenRecorder {
    pub async fn new(
        database: Database,
        interval: Duration,
        sender: mpsc::UnboundedSender<WorkItem>,
        passphrase: SecretString,
        configuration: Configuration,
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
            configuration,
        })
    }

    async fn capture_active_window(&self) -> Result<CapturedScreenshot> {
        let filter = CapturableContentFilter::EVERYTHING_NORMAL;
        let content = CapturableContent::new(filter).await?;
        // supported by both windows and macos
        let format = CapturePixelFormat::Bgra8888;

        let active_window = active_win_pos_rs::get_active_window()
            .map_err(|_| eyre!("unable to get active window"))?;
        let window = content
            .windows()
            .find(|w| w.application().pid() == active_window.process_id as i32)
            .ok_or_eyre("could not find active window")?;
        let app_name = window.application().name();
        let title = window.title();

        let config = CaptureConfig::with_window(window, format)?;
        let video_frame = screenshot::take_screenshot(self.access_token.clone(), config).await?;
        let bitmap = video_frame.get_bitmap()?;
        Ok(CapturedScreenshot {
            bitmap,
            app_name,
            title,
        })
    }

    async fn should_save_screenshot(&self, screenshot: &RgbImage) -> Result<bool> {
        let last_screenshot = self.database.find_most_recent_screenshot().await?;
        match last_screenshot {
            Some(last_screenshot) => {
                let last_image = last_screenshot.load_image(&self.passphrase)?;
                let last_image = DynamicImage::from(last_image);
                let screenshot = DynamicImage::ImageRgb8(screenshot.clone());
                Ok(!is_similar(&last_image, &screenshot))
            }
            None => Ok(true),
        }
    }

    async fn create_screenshot(&self) -> Result<Option<Screenshot>> {
        let CapturedScreenshot {
            bitmap,
            app_name,
            title,
        } = self.capture_active_window().await?;
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

        if !self.should_save_screenshot(&image).await? {
            return Ok(None);
        }

        let timestamp = OffsetDateTime::now_utc().unix_timestamp();
        let path = self
            .configuration
            .screenshot_directory
            .join(format!("{}.jpeg.enc", timestamp));
        let mut bytes = Cursor::new(vec![]);
        image.write_to(&mut bytes, ImageFormat::Jpeg)?;
        encrypt_file(&path, self.passphrase.clone(), bytes.get_ref())?;
        let screenshot = NewScreenshot {
            path: path.to_string(),
            timestamp: OffsetDateTime::now_utc(),
            window_title: title,
            application_name: app_name,
        };

        let screenshot = self.database.insert(screenshot).await?;

        Ok(Some(screenshot))
    }

    pub async fn start(&self) -> Result<()> {
        info!("starting screen recorder");
        loop {
            if let Some(screenshot) = self.create_screenshot().await? {
                self.sender.send(WorkItem { screenshot })?;
                tokio::time::sleep(self.interval).await;
            } else {
                info!("screenshots are too similar, skipping");
            }
        }
    }
}
