use age::secrecy::SecretString;
use color_eyre::Result;
use ollama_rs::generation::completion::request::GenerationRequest;
use ollama_rs::generation::images::Image;
use ollama_rs::Ollama;

use crate::database::Screenshot;
use crate::encryption;

const MODEL_NAME: &str = "llava-llama3";

pub async fn generate_description(
    screenshot: &Screenshot,
    passphrase: &SecretString,
) -> Result<String> {
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine as _;

    let ollama = Ollama::default();
    let bytes = encryption::decrypt_file(&screenshot.path, &passphrase).await?;
    let base64 = STANDARD.encode(bytes);
    let request = GenerationRequest::new(
        MODEL_NAME.into(),
        "The following image is a screenshot from a MacOS computer. Describe its contents using keywords, separated by comams.".into(),
    )
    .images(vec![Image::from_base64(&base64)]);

    let response = ollama.generate(request).await?;
    Ok(response.response)
}
