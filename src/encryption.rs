use std::{
    fs::{self, File},
    io::{Read, Write},
};

use age::{secrecy::Secret, stream::StreamReader, Decryptor, Encryptor};
use camino::Utf8Path;
use color_eyre::Result;

/// Encrypt arbitrary bytes and write them to a file.
pub fn encrypt_file(bytes: &[u8], path: impl AsRef<Utf8Path>, passphrase: &str) -> Result<()> {
    let encrypted = {
        let encryptor = Encryptor::with_user_passphrase(Secret::new(passphrase.to_owned()));

        let mut encrypted = vec![];
        let mut writer = encryptor.wrap_output(&mut encrypted)?;
        writer.write_all(bytes)?;
        writer.finish()?;

        encrypted
    };

    fs::write(path.as_ref(), encrypted)?;
    Ok(())
}

pub fn decrypt_file_stream(
    path: impl AsRef<Utf8Path>,
    passphrase: &str,
) -> Result<StreamReader<File>> {
    let encrypted = File::open(path.as_ref())?;

    let decryptor = match Decryptor::new(encrypted)? {
        Decryptor::Passphrase(d) => d,
        _ => unreachable!(),
    };

    decryptor
        .decrypt(&Secret::new(passphrase.to_owned()), None)
        .map_err(From::from)
}

/// Decrypt a file and return the decrypted bytes.
pub fn decrypt_file(path: impl AsRef<Utf8Path>, passphrase: &str) -> Result<Vec<u8>> {
    let mut reader = decrypt_file_stream(path, passphrase)?;
    let mut decrypted = vec![];
    reader.read_to_end(&mut decrypted)?;
    Ok(decrypted)
}
