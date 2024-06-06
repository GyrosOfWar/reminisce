use std::{
    fs::{self, File},
    io::{Read, Seek, Write},
};

use age::{secrecy::SecretString, stream::StreamReader, Decryptor, Encryptor};
use camino::Utf8Path;
use color_eyre::Result;

pub fn encrypted_writer(
    path: impl AsRef<Utf8Path>,
    passphrase: SecretString,
) -> Result<impl Write + Seek> {
    let encryptor = Encryptor::with_user_passphrase(passphrase);
    let inner_writer = File::create(path.as_ref())?;
    let writer = encryptor.wrap_output(inner_writer)?;
    Ok(writer)
}

pub fn decrypt_file_stream(
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
pub fn decrypt_file(path: impl AsRef<Utf8Path>, passphrase: &SecretString) -> Result<Vec<u8>> {
    let mut reader = decrypt_file_stream(path, passphrase)?;
    let mut decrypted = vec![];
    reader.read_to_end(&mut decrypted)?;
    Ok(decrypted)
}

pub fn get_passphrase() -> Result<SecretString> {
    let input = rpassword::prompt_password("Enter your passphrase: ")?;
    Ok(SecretString::new(input))
}
