use crate::transport::crypto;

#[test]
fn encrypt_decrypt_roundtrip() {
    let plaintext = "v=0\r\no=- 123 2 IN IP4 0.0.0.0\r\ns=-\r\n";
    let passphrase = "test-phrase-42";

    let code = crypto::encrypt(plaintext, passphrase).unwrap();
    let decrypted = crypto::decrypt(&code, passphrase).unwrap();

    assert_eq!(decrypted, plaintext);
}

#[test]
fn wrong_passphrase_fails() {
    let plaintext = "secret sdp data";
    let code = crypto::encrypt(plaintext, "correct-pass").unwrap();
    let result = crypto::decrypt(&code, "wrong-pass");

    assert!(result.is_err());
}

#[test]
fn each_encryption_produces_unique_code() {
    let plaintext = "same input";
    let passphrase = "same-pass";

    let code1 = crypto::encrypt(plaintext, passphrase).unwrap();
    let code2 = crypto::encrypt(plaintext, passphrase).unwrap();

    assert_ne!(code1, code2, "random salt+iv should produce different codes");

    assert_eq!(crypto::decrypt(&code1, passphrase).unwrap(), plaintext);
    assert_eq!(crypto::decrypt(&code2, passphrase).unwrap(), plaintext);
}

#[test]
fn invalid_code_returns_error() {
    assert!(crypto::decrypt("not-base64!!!", "pass").is_err());
    assert!(crypto::decrypt("dG9vc2hvcnQ", "pass").is_err()); // too short
    assert!(crypto::decrypt("", "pass").is_err());
}

#[test]
fn tampered_code_fails_decryption() {
    let code = crypto::encrypt("real data", "pass").unwrap();
    let mut bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(&code)
        .unwrap();

    if let Some(last) = bytes.last_mut() {
        *last ^= 0xFF;
    }

    let tampered = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&bytes);
    assert!(crypto::decrypt(&tampered, "pass").is_err());
}

#[test]
fn passphrase_is_human_readable() {
    let pass = crypto::generate_passphrase().unwrap();
    let parts: Vec<&str> = pass.split('-').collect();

    assert_eq!(parts.len(), 3, "format: word-word-NN");
    assert!(parts[0].chars().all(|c| c.is_ascii_alphabetic()));
    assert!(parts[1].chars().all(|c| c.is_ascii_alphabetic()));
    assert!(parts[2].len() == 2);
    assert!(parts[2].chars().all(|c| c.is_ascii_digit()));
}

#[test]
fn generated_passphrases_vary() {
    let p1 = crypto::generate_passphrase().unwrap();
    let p2 = crypto::generate_passphrase().unwrap();
    let p3 = crypto::generate_passphrase().unwrap();

    // Extremely unlikely all three are equal
    assert!(p1 != p2 || p2 != p3, "passphrases should be random");
}

use base64::Engine;
