use crate::{database::Screenshot, encryption};
use age::secrecy::SecretString;
use color_eyre::Result;
use ollama_rs::{
    generation::{completion::request::GenerationRequest, images::Image},
    Ollama,
};

const MODEL_NAME: &str = "llava-llama3";

pub async fn generate_description(
    screenshot: &Screenshot,
    passphrase: &SecretString,
) -> Result<String> {
    use base64::{engine::general_purpose::STANDARD, Engine as _};

    let ollama = Ollama::default();
    let bytes = encryption::decrypt_file(&screenshot.path, &passphrase)?;
    let base64 = STANDARD.encode(bytes);
    let request = GenerationRequest::new(
        MODEL_NAME.into(),
        "The following image is a screenshot from a MacOS computer. Describe its contents using keywords, separated by comams.".into(),
    )
    .images(vec![Image::from_base64(&base64)]);

    let response = ollama.generate(request).await?;
    Ok(response.response)
}
