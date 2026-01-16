use std::path::PathBuf;
use thiserror::Error;

/// Main error type for Denver application
#[derive(Error, Debug)]
pub enum DenverError {
    #[error("Failed to read file '{path}': {source}")]
    FileRead {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Failed to write file '{path}': {source}")]
    FileWrite {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Failed to scan directory '{path}': {source}")]
    DirectoryScan {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Invalid .env file format at line {line}: {message}")]
    ParseError { line: usize, message: String },

    #[error("Project '{name}' not found")]
    ProjectNotFound { name: String },

    #[error("Key '{key}' not found")]
    KeyNotFound { key: String },

    #[error("Key '{key}' already exists")]
    KeyExists { key: String },

    #[error("Invalid key name '{key}': {reason}")]
    InvalidKeyName { key: String, reason: String },

    #[error("Terminal error: {0}")]
    Terminal(#[from] std::io::Error),
}

/// Result type alias for Denver operations
pub type DenverResult<T> = Result<T, DenverError>;

/// Validate an environment variable key name
pub fn validate_key(key: &str) -> Result<(), DenverError> {
    if key.is_empty() {
        return Err(DenverError::InvalidKeyName {
            key: key.to_string(),
            reason: "Key cannot be empty".to_string(),
        });
    }

    let first = key.chars().next().unwrap();
    if !first.is_ascii_alphabetic() && first != '_' {
        return Err(DenverError::InvalidKeyName {
            key: key.to_string(),
            reason: "Key must start with a letter or underscore".to_string(),
        });
    }

    if !key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(DenverError::InvalidKeyName {
            key: key.to_string(),
            reason: "Key can only contain letters, numbers, and underscores".to_string(),
        });
    }

    Ok(())
}
