use std::{
    fs::File,
    io::{Read, Write},
};

use age::{
    secrecy::SecretString,
    stream::{StreamReader, StreamWriter},
    Decryptor, Encryptor,
};
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

pub fn encrypt_file(
    path: impl AsRef<Utf8Path>,
    passphrase: SecretString,
    data: &[u8],
) -> Result<()> {
    tokio::task::block_in_place(|| {
        let mut writer = encrypting_writer(path, passphrase)?;
        writer.write_all(data)?;
        writer.finish()?;
        Ok(())
    })
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
pub fn decrypt_file(path: impl AsRef<Utf8Path>, passphrase: &SecretString) -> Result<Vec<u8>> {
    tokio::task::block_in_place(|| {
        let mut reader = decrypting_reader(path, passphrase)?;
        let mut decrypted = vec![];
        reader.read_to_end(&mut decrypted)?;
        Ok(decrypted)
    })
}

pub fn get_passphrase() -> Result<SecretString> {
    let input = rpassword::prompt_password("Enter your passphrase: ")?;
    Ok(SecretString::new(input))
}
