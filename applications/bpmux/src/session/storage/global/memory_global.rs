use std::collections::HashMap;
use std::sync::Mutex;

use super::GlobalStore;

pub struct MemoryGlobalStore {
    identity: Mutex<HashMap<String, crate::memory::identity::IdentityIndex>>,
    briefings: Mutex<HashMap<String, String>>,
    tokens: Mutex<Vec<crate::telemetry::tokens::TokenRow>>,
    usage: Mutex<HashMap<String, crate::telemetry::usage::Record>>,
}

impl Default for MemoryGlobalStore {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryGlobalStore {
    pub fn new() -> Self {
        Self {
            identity: Mutex::new(HashMap::new()),
            briefings: Mutex::new(HashMap::new()),
            tokens: Mutex::new(Vec::new()),
            usage: Mutex::new(HashMap::new()),
        }
    }
}

impl GlobalStore for MemoryGlobalStore {
    fn load_identity(&self, role: &str) -> crate::memory::identity::IdentityIndex {
        self.identity
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(role)
            .cloned()
            .unwrap_or_default()
    }

    fn save_identity(
        &self,
        role: &str,
        index: &crate::memory::identity::IdentityIndex,
    ) -> Result<(), std::io::Error> {
        self.identity
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(role.to_string(), index.clone());
        Ok(())
    }

    fn load_briefing(&self, role: &str) -> String {
        self.briefings
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(role)
            .cloned()
            .unwrap_or_default()
    }

    fn save_briefing(&self, role: &str, content: &str) -> Result<(), std::io::Error> {
        self.briefings
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(role.to_string(), content.to_string());
        Ok(())
    }

    fn append_token_row(
        &self,
        row: &crate::telemetry::tokens::TokenRow,
    ) -> Result<(), crate::session::log::LogError> {
        self.tokens
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(row.clone());
        Ok(())
    }

    fn read_token_rows(
        &self,
    ) -> Result<Vec<crate::telemetry::tokens::TokenRow>, crate::session::log::LogError> {
        Ok(self
            .tokens
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone())
    }

    fn load_usage(&self) -> HashMap<String, crate::telemetry::usage::Record> {
        self.usage.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    fn save_usage(
        &self,
        map: &HashMap<String, crate::telemetry::usage::Record>,
    ) -> Result<(), std::io::Error> {
        *self.usage.lock().unwrap_or_else(|e| e.into_inner()) = map.clone();
        Ok(())
    }

    fn ensure_auth_token(&self) -> std::io::Result<String> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "MemoryGlobalStore does not support auth_token (process-owned secret \
             requires mode-0600 filesystem semantics); use FileGlobalStore with \
             BPMUX_AUTH_TOKEN_PATH for test isolation",
        ))
    }

    fn read_auth_token(&self) -> std::io::Result<String> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "MemoryGlobalStore does not support auth_token (process-owned secret \
             requires mode-0600 filesystem semantics); use FileGlobalStore with \
             BPMUX_AUTH_TOKEN_PATH for test isolation",
        ))
    }
}
