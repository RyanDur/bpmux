use aes_gcm::aead::Aead;
use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use ring::pbkdf2;
use ring::rand::{SecureRandom, SystemRandom};
use std::num::NonZeroU32;

const SALT_LEN: usize = 16;
const IV_LEN: usize = 12;
const KEY_LEN: usize = 32;
const PBKDF2_ITERATIONS: u32 = 100_000;

#[derive(Debug)]
pub enum CryptoError {
    KeyDerivation,
    Encryption,
    Decryption,
    InvalidCode,
    RandomGeneration,
}

impl std::fmt::Display for CryptoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::KeyDerivation => write!(f, "key derivation failed"),
            Self::Encryption => write!(f, "encryption failed"),
            Self::Decryption => write!(f, "decryption failed (wrong passphrase?)"),
            Self::InvalidCode => write!(f, "invalid connection code"),
            Self::RandomGeneration => write!(f, "random number generation failed"),
        }
    }
}

impl std::error::Error for CryptoError {}

const ITERATIONS: NonZeroU32 = match NonZeroU32::new(PBKDF2_ITERATIONS) {
    Some(n) => n,
    None => unreachable!(),
};

fn derive_key(passphrase: &str, salt: &[u8]) -> [u8; KEY_LEN] {
    let mut key = [0u8; KEY_LEN];
    pbkdf2::derive(
        pbkdf2::PBKDF2_HMAC_SHA256,
        ITERATIONS,
        salt,
        passphrase.as_bytes(),
        &mut key,
    );
    key
}

pub fn encrypt(plaintext: &str, passphrase: &str) -> Result<String, CryptoError> {
    let rng = SystemRandom::new();

    let mut salt = [0u8; SALT_LEN];
    rng.fill(&mut salt)
        .map_err(|_| CryptoError::RandomGeneration)?;

    let mut iv = [0u8; IV_LEN];
    rng.fill(&mut iv)
        .map_err(|_| CryptoError::RandomGeneration)?;

    let key = derive_key(passphrase, &salt);
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|_| CryptoError::KeyDerivation)?;
    let nonce = Nonce::from_slice(&iv);

    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|_| CryptoError::Encryption)?;

    let mut combined = Vec::with_capacity(SALT_LEN + IV_LEN + ciphertext.len());
    combined.extend_from_slice(&salt);
    combined.extend_from_slice(&iv);
    combined.extend_from_slice(&ciphertext);

    Ok(URL_SAFE_NO_PAD.encode(combined))
}

pub fn decrypt(code: &str, passphrase: &str) -> Result<String, CryptoError> {
    let combined = URL_SAFE_NO_PAD
        .decode(code)
        .map_err(|_| CryptoError::InvalidCode)?;

    let min_len = SALT_LEN + IV_LEN + 1;
    if combined.len() < min_len {
        return Err(CryptoError::InvalidCode);
    }

    let salt = &combined[..SALT_LEN];
    let iv = &combined[SALT_LEN..SALT_LEN + IV_LEN];
    let ciphertext = &combined[SALT_LEN + IV_LEN..];

    let key = derive_key(passphrase, salt);
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|_| CryptoError::KeyDerivation)?;
    let nonce = Nonce::from_slice(iv);

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| CryptoError::Decryption)?;

    String::from_utf8(plaintext).map_err(|_| CryptoError::Decryption)
}

pub fn generate_passphrase() -> Result<String, CryptoError> {
    let rng = SystemRandom::new();
    let mut bytes = [0u8; 6];
    rng.fill(&mut bytes)
        .map_err(|_| CryptoError::RandomGeneration)?;

    let words = [
        "alpha", "blue", "coral", "delta", "echo", "frost", "green", "haze", "iron", "jade",
        "keen", "lime", "mist", "nova", "opal", "peak", "quartz", "reef", "storm", "tide", "ultra",
        "vine", "wind", "xenon", "yield", "zinc", "amber", "birch", "cedar", "drift", "ember",
        "fern",
    ];

    let w1 = words[bytes[0] as usize % words.len()];
    let w2 = words[bytes[1] as usize % words.len()];
    let num = ((bytes[2] as u16) << 8 | bytes[3] as u16) % 100;

    Ok(format!("{w1}-{w2}-{num:02}"))
}
