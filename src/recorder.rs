use std::io::Cursor;
use std::time::Duration;

use age::secrecy::SecretString;
use color_eyre::eyre::{eyre, OptionExt};
use color_eyre::Result;
use crabgrab::capturable_content::{CapturableContent, CapturableContentFilter};
use crabgrab::capture_stream::{
    CaptureAccessToken, CaptureConfig, CapturePixelFormat, CaptureStream,
};
use crabgrab::feature::screenshot;
use crabgrab::prelude::{BoxedSliceFrameBitmap, FrameBitmap, VideoFrameBitmap};
use image::{DynamicImage, ImageBuffer, ImageFormat, RgbImage, RgbaImage};
use time::OffsetDateTime;
use tokio::sync::mpsc;
use tracing::{info, instrument, trace};

use crate::configuration::Configuration;
use crate::database::{Database, NewScreenshot, Screenshot};
use crate::encryption::encrypt_file;
use crate::image_processing::similarity::is_similar;
use crate::queue::WorkItem;

#[derive(Debug, Clone, Copy)]
enum CaptureType {
    Screen,
    ActiveWindow,
}

struct CapturedScreenshot {
    bitmap: BoxedSliceFrameBitmap,
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

    #[instrument(skip(self))]
    async fn capture(&self, capture_type: CaptureType) -> Result<CapturedScreenshot> {
        let filter = CapturableContentFilter::EVERYTHING_NORMAL;
        let content = CapturableContent::new(filter).await?;
        // supported by both windows and macos
        let format = CapturePixelFormat::Bgra8888;

        let active_window = active_win_pos_rs::get_active_window()
            .map_err(|_| eyre!("unable to get active window"))?;
        info!("active window: {:?}", active_window);

        let window = content
            .windows()
            .find(|w| w.application().pid() == active_window.process_id as i32)
            .ok_or_eyre("could not find active window")?;

        let app_name = window.application().name();
        let title = window.title();
        info!("capturing window: {} - {}", app_name, title);

        let config = match capture_type {
            CaptureType::Screen => {
                CaptureConfig::with_display(content.displays().next().unwrap(), format)
            }
            CaptureType::ActiveWindow => CaptureConfig::with_window(window, format)?,
        };

        let video_frame = screenshot::take_screenshot(self.access_token, config).await?;
        let bitmap = video_frame.get_bitmap()?;
        Ok(CapturedScreenshot {
            bitmap,
            app_name,
            title,
        })
    }

    #[instrument(skip(self, screenshot))]
    async fn should_save_screenshot(&self, screenshot: &RgbImage) -> Result<bool> {
        let last_screenshot = self.database.find_most_recent_screenshot().await?;
        match last_screenshot {
            Some(last_screenshot) => {
                let last_image = last_screenshot.load_image(&self.passphrase).await?;
                let last_image = DynamicImage::from(last_image);
                let screenshot = DynamicImage::ImageRgb8(screenshot.clone());
                let is_similar = is_similar(&last_image, &screenshot)?;
                Ok(!is_similar)
            }
            None => Ok(true),
        }
    }

    #[instrument(skip(self))]
    async fn create_screenshot(&self) -> Result<Option<Screenshot>> {
        let CapturedScreenshot {
            bitmap,
            app_name,
            title,
        } = self.capture(CaptureType::ActiveWindow).await?;
        let pixels = match bitmap {
            FrameBitmap::BgraUnorm8x4(frame) => frame,
            FrameBitmap::ArgbUnormPacked2101010(frame) => unimplemented!(),
            FrameBitmap::RgbaF16x4(frame) => unimplemented!(),
            FrameBitmap::YCbCr(frame) => unimplemented!(),
        };

        let data: Vec<_> = pixels
            .data
            .iter()
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
        encrypt_file(&path, self.passphrase.clone(), bytes.into_inner()).await?;
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
            match self.create_screenshot().await {
                Ok(Some(screenshot)) => {
                    self.sender.send(WorkItem { screenshot })?;
                }
                Ok(None) => {
                    info!("screenshots are too similar, skipping");
                }
                Err(e) => {
                    info!("error creating screenshot: {e}");
                }
            }

            trace!("sleeping for {} seconds", self.interval.as_secs_f64());
            tokio::time::sleep(self.interval).await;
        }
    }
}
