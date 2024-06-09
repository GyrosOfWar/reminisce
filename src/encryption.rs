use std::fs::File;
use std::io::{Read, Write};

use age::secrecy::SecretString;
use age::stream::{StreamReader, StreamWriter};
use age::{Decryptor, Encryptor};
use camino::Utf8Path;
use color_eyre::Result;

fn encrypting_writer(
    path: impl AsRef<Utf8Path>,
    passphrase: SecretString,
) -> Result<StreamWriter<File>> {
    let encryptor = Encryptor::with_user_passphrase(passphrase);
    let inner_writer = File::create(path.as_ref())?;
    let writer = encryptor.wrap_output(inner_writer)?;
    Ok(writer)
}

pub async fn encrypt_file(
    path: impl AsRef<Utf8Path>,
    passphrase: SecretString,
    data: Vec<u8>,
) -> Result<()> {
    let path = path.as_ref().to_path_buf();
    tokio::task::spawn_blocking(move || {
        let mut writer = encrypting_writer(path, passphrase)?;
        writer.write_all(&data)?;
        writer.finish()?;
        Ok(())
    })
    .await?
}

fn decrypting_reader(
    path: impl AsRef<Utf8Path>,
    passphrase: &SecretString,
) -> Result<StreamReader<File>> {
    let encrypted = File::open(path.as_ref())?;

    let decryptor = match Decryptor::new(encrypted)? {
        Decryptor::Passphrase(d) => d,
        _ => unreachable!(),
    };

    decryptor.decrypt(passphrase, None).map_err(From::from)
}

/// Decrypt a file and return the decrypted bytes.
pub async fn decrypt_file(
    path: impl AsRef<Utf8Path>,
    passphrase: &SecretString,
) -> Result<Vec<u8>> {
    let path = path.as_ref().to_path_buf();
    let passphrase = passphrase.clone();
    tokio::task::spawn_blocking(move || {
        let mut reader = decrypting_reader(path, &passphrase)?;
        let mut decrypted = vec![];
        reader.read_to_end(&mut decrypted)?;
        Ok(decrypted)
    })
    .await?
}

pub fn get_passphrase(prompt: &str) -> Result<SecretString> {
    let input = rpassword::prompt_password(prompt)?;
    Ok(SecretString::new(input))
}
