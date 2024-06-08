use crate::database::Screenshot;
use age::secrecy::SecretString;
use color_eyre::{eyre::eyre, Result};
use ocrs::{ImageSource, OcrEngine, OcrEngineParams};

pub fn extract_text(screenshot: &Screenshot, passphrase: &SecretString) -> Result<String> {
    let mut params = OcrEngineParams::default();
    params.recognition_model = Some(rten::Model::load_file("models/text-recognition.rten")?);
    params.detection_model = Some(rten::Model::load_file("models/text-detection.rten")?);
    let engine = OcrEngine::new(params).map_err(|e| eyre!("Failed to create engine: {}", e))?;
    let image = screenshot.load_image(passphrase)?;
    let img_source = ImageSource::from_bytes(image.as_raw(), image.dimensions())?;
    let input = engine
        .prepare_input(img_source)
        .map_err(|e| eyre!("Failed to prepare input: {}", e))?;
    let text = engine
        .get_text(&input)
        .map_err(|e| eyre!("Failed to get text: {}", e))?;
    Ok(text)
}
