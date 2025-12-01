use crate::errors::{AppError, ConfigError};
use std::path::PathBuf;

/// Get the user's home directory
pub fn get_home_dir() -> Result<PathBuf, AppError> {
    dirs::home_dir()
        .ok_or_else(|| {
            ConfigError::DirectoryCreationFailed {
                path: PathBuf::from("~"),
            }
            .into()
        })
}


