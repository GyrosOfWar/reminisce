use age::secrecy::SecretString;
use color_eyre::eyre::eyre;
use color_eyre::Result;
use ocrs::{ImageSource, OcrEngine, OcrEngineParams};
use rten::Model;

use crate::database::Screenshot;

pub async fn extract_text(screenshot: &Screenshot, passphrase: &SecretString) -> Result<String> {
    let params = OcrEngineParams {
        detection_model: Some(Model::load_file("models/text-detection.rten")?),
        recognition_model: Some(Model::load_file("models/text-recognition.rten")?),
        ..Default::default()
    };

    let engine = OcrEngine::new(params).map_err(|e| eyre!("Failed to create engine: {}", e))?;
    let image = screenshot.load_image(passphrase).await?;
    let img_source = ImageSource::from_bytes(image.as_raw(), image.dimensions())?;
    let input = engine
        .prepare_input(img_source)
        .map_err(|e| eyre!("Failed to prepare input: {}", e))?;
    let text = engine
        .get_text(&input)
        .map_err(|e| eyre!("Failed to get text: {}", e))?;
    Ok(text)
}
