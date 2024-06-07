use crate::database::Screenshot;
use age::secrecy::SecretString;
use color_eyre::{eyre::eyre, Result};
use ocrs::{ImageSource, OcrEngine, OcrEngineParams};

pub fn extract_text(screenshot: &Screenshot, passphrase: &SecretString) -> Result<String> {
    let params = OcrEngineParams::default();
    let engine = OcrEngine::new(params).map_err(|e| eyre!("Failed to create engine: {}", e))?;
    let bytes = screenshot.load_image(passphrase)?;
    let image = image::load_from_memory_with_format(&bytes, image::ImageFormat::Jpeg)?.into_rgb8();
    let img_source = ImageSource::from_bytes(image.as_raw(), image.dimensions())?;
    let input = engine
        .prepare_input(img_source)
        .map_err(|e| eyre!("Failed to prepare input: {}", e))?;
    let text = engine
        .get_text(&input)
        .map_err(|e| eyre!("Failed to get text: {}", e))?;
    Ok(text)
}
