#[derive(Debug)]
pub enum PersistError {
    Io(std::io::Error),
    Serde(serde_json::Error),
}

impl std::fmt::Display for PersistError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PersistError::Io(e) => write!(f, "i/o: {e}"),
            PersistError::Serde(e) => write!(f, "serde: {e}"),
        }
    }
}

impl std::error::Error for PersistError {}

impl From<std::io::Error> for PersistError {
    fn from(e: std::io::Error) -> Self {
        PersistError::Io(e)
    }
}

impl From<serde_json::Error> for PersistError {
    fn from(e: serde_json::Error) -> Self {
        PersistError::Serde(e)
    }
}
