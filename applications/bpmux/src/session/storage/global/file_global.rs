use std::path::PathBuf;

use super::GlobalStore;

pub struct FileGlobalStore {
    identity_dir: Option<PathBuf>,
    briefings_dir: Option<PathBuf>,
    tokens_log_path: Option<PathBuf>,
    usage_path: Option<PathBuf>,
    auth_token_path: Option<PathBuf>,
}

impl Default for FileGlobalStore {
    fn default() -> Self {
        Self::new()
    }
}

impl FileGlobalStore {
    pub fn new() -> Self {
        Self {
            identity_dir: None,
            briefings_dir: None,
            tokens_log_path: None,
            usage_path: None,
            auth_token_path: None,
        }
    }

    pub fn with_identity_dir(mut self, dir: PathBuf) -> Self {
        self.identity_dir = Some(dir);
        self
    }

    pub fn with_briefings_dir(mut self, dir: PathBuf) -> Self {
        self.briefings_dir = Some(dir);
        self
    }

    pub fn with_tokens_log_path(mut self, path: PathBuf) -> Self {
        self.tokens_log_path = Some(path);
        self
    }

    pub fn with_usage_path(mut self, path: PathBuf) -> Self {
        self.usage_path = Some(path);
        self
    }

    pub fn with_auth_token_path(mut self, path: PathBuf) -> Self {
        self.auth_token_path = Some(path);
        self
    }

    fn resolve_identity_dir(&self) -> Result<PathBuf, std::io::Error> {
        if let Some(d) = &self.identity_dir {
            return Ok(d.clone());
        }
        crate::memory::identity::default_identity_dir()
    }

    fn resolve_briefings_dir(&self) -> PathBuf {
        if let Some(d) = &self.briefings_dir {
            return d.clone();
        }
        crate::session::briefing::briefings_dir()
    }

    fn resolve_tokens_log_path(&self) -> Result<PathBuf, crate::session::log::LogError> {
        if let Some(p) = &self.tokens_log_path {
            return Ok(p.clone());
        }
        crate::telemetry::tokens::default_tokens_log_path()
    }

    fn resolve_usage_path(&self) -> Option<PathBuf> {
        if let Some(p) = &self.usage_path {
            return Some(p.clone());
        }
        crate::telemetry::usage::default_usage_path()
    }

    fn resolve_auth_token_path(&self) -> std::io::Result<PathBuf> {
        if let Some(p) = &self.auth_token_path {
            return Ok(p.clone());
        }
        crate::auth::default_auth_token_path().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "home directory not found for auth_token",
            )
        })
    }
}

impl GlobalStore for FileGlobalStore {
    fn load_identity(&self, role: &str) -> crate::memory::identity::IdentityIndex {
        match self.resolve_identity_dir() {
            Ok(dir) => crate::memory::identity::load_identity(&dir, role),
            Err(_) => crate::memory::identity::IdentityIndex::default(),
        }
    }

    fn save_identity(
        &self,
        role: &str,
        index: &crate::memory::identity::IdentityIndex,
    ) -> Result<(), std::io::Error> {
        let dir = self.resolve_identity_dir()?;
        crate::memory::identity::save_identity(&dir, role, index)
    }

    fn load_briefing(&self, role: &str) -> String {
        crate::session::briefing::load_briefing_in(&self.resolve_briefings_dir(), role)
    }

    fn save_briefing(&self, role: &str, content: &str) -> Result<(), std::io::Error> {
        crate::session::briefing::save_briefing_in(&self.resolve_briefings_dir(), role, content)
    }

    fn append_token_row(
        &self,
        row: &crate::telemetry::tokens::TokenRow,
    ) -> Result<(), crate::session::log::LogError> {
        let path = self.resolve_tokens_log_path()?;
        crate::telemetry::tokens::append_token_row(&path, row)
    }

    fn read_token_rows(
        &self,
    ) -> Result<Vec<crate::telemetry::tokens::TokenRow>, crate::session::log::LogError> {
        let path = self.resolve_tokens_log_path()?;
        crate::telemetry::tokens::read_token_rows(&path)
    }

    fn load_usage(&self) -> std::collections::HashMap<String, crate::telemetry::usage::Record> {
        let Some(path) = self.resolve_usage_path() else {
            return std::collections::HashMap::new();
        };
        crate::telemetry::usage::load_from(&path)
    }

    fn save_usage(
        &self,
        map: &std::collections::HashMap<String, crate::telemetry::usage::Record>,
    ) -> Result<(), std::io::Error> {
        let Some(path) = self.resolve_usage_path() else {
            return Ok(());
        };
        crate::telemetry::usage::save_to(&path, map)
    }

    fn ensure_auth_token(&self) -> std::io::Result<String> {
        let path = self.resolve_auth_token_path()?;
        crate::auth::ensure(&path)
    }

    fn read_auth_token(&self) -> std::io::Result<String> {
        let path = self.resolve_auth_token_path()?;
        crate::auth::read(&path)
    }
}
