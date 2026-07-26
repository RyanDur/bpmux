use std::collections::HashMap;
use std::sync::Mutex;

use crate::github::connection::{SecretError, SecretStore};

#[derive(Default)]
pub struct InMemorySecretStore {
    entries: Mutex<HashMap<String, String>>,
    touches: Mutex<u32>,
}

impl InMemorySecretStore {
    pub fn touches(&self) -> u32 {
        *self.touches.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn touch(&self) {
        *self.touches.lock().unwrap_or_else(|e| e.into_inner()) += 1;
    }
}

impl SecretStore for InMemorySecretStore {
    fn store(&self, key: &str, value: &str) -> Result<(), SecretError> {
        self.touch();
        self.entries
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(key.to_string(), value.to_string());
        Ok(())
    }

    fn load(&self, key: &str) -> Result<Option<String>, SecretError> {
        self.touch();
        Ok(self
            .entries
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(key)
            .cloned())
    }

    fn delete(&self, key: &str) -> Result<(), SecretError> {
        self.touch();
        self.entries
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(key);
        Ok(())
    }
}
