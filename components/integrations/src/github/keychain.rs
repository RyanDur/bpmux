use keyring::Entry;

use super::connection::{SecretError, SecretStore};

const SERVICE: &str = "bpmux";

pub struct KeychainSecretStore;

impl KeychainSecretStore {
    fn entry(key: &str) -> Result<Entry, SecretError> {
        Entry::new(SERVICE, key).map_err(|e| SecretError(e.to_string()))
    }
}

impl SecretStore for KeychainSecretStore {
    fn store(&self, key: &str, value: &str) -> Result<(), SecretError> {
        Self::entry(key)?
            .set_password(value)
            .map_err(|e| SecretError(e.to_string()))
    }

    fn load(&self, key: &str) -> Result<Option<String>, SecretError> {
        match Self::entry(key)?.get_password() {
            Ok(value) => Ok(Some(value)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(SecretError(e.to_string())),
        }
    }

    fn delete(&self, key: &str) -> Result<(), SecretError> {
        match Self::entry(key)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(SecretError(e.to_string())),
        }
    }
}
