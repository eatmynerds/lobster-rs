#[derive(Debug)]
pub enum EncryptionError {
    OpenSSLError(openssl::error::ErrorStack),
    ParsingError,
}

/// Decrypt Encoded Url Sources
/// # Parameters
/// * `encrypted_url` - The AES-256-CBC Encrypted url.
/// * `secret` - the decryption key as bytes.
pub fn decrypt_url(encrypted_url: &str, key: &[u8]) -> Result<String, EncryptionError> {
    let decoded_ciphertext =
        openssl::base64::decode_block(encrypted_url).map_err(EncryptionError::OpenSSLError)?;

    assert_eq!(&decoded_ciphertext[0..8], "Salted__".as_bytes());

    let cipher = openssl::symm::Cipher::aes_256_cbc();
    let key_iv_pair = openssl::pkcs5::bytes_to_key(
        cipher,
        openssl::hash::MessageDigest::md5(),
        key,
        Some(&decoded_ciphertext[8..16]),
        1,
    )
    .map_err(EncryptionError::OpenSSLError)?;

    String::from_utf8(
        openssl::symm::decrypt(
            cipher,
            &key_iv_pair.key,
            key_iv_pair.iv.as_ref().map(|value| value.as_ref()),
            &decoded_ciphertext[16..],
        )
        .map_err(EncryptionError::OpenSSLError)?,
    )
    .map_err(|_| EncryptionError::ParsingError)
}

pub fn key_finder(encrypted_url: &mut String, key_resolver: Vec<(u8, u8)>) -> String {
    let rough_copy: String = encrypted_url.clone();

    key_resolver
        .iter()
        .map(|(start, end)| {
            let segment = &rough_copy[(*start as usize)..(*end as usize)];
            *encrypted_url = encrypted_url.replace(segment, "");
            segment
        })
        .collect::<String>()
}