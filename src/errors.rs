use std::io;
use std::path::PathBuf;
use thiserror::Error;

/// Main error type for git-commit-gen
#[derive(Error, Debug)]
pub enum AppError {
    #[error("Git error: {0}")]
    Git(#[from] GitError),

    #[error("Config error: {0}")]
    Config(#[from] ConfigError),

    #[error("Handler error: {0}")]
    Handler(#[from] HandlerError),

    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("UTF-8 error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),

    #[error("HTTP error: {0}")]
    HttpError(String),

    #[error("Editor error: {0}")]
    EditorError(String),
}

/// Git operation errors
#[derive(Error, Debug)]
pub enum GitError {
    #[error("Git is not available. Please install git first.")]
    NotAvailable,

    #[error("Not a git repository. Please run this command in a git repository.")]
    NotRepository,

    #[error("No staged changes found. Please stage some files first with 'git add'.")]
    NoStagedChanges,

    #[error("Git command failed: {command}")]
    CommandFailed { command: String, stderr: String },

    #[error("Failed to parse git output: {0}")]
    ParseError(String),
}

/// Configuration errors
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Config file already exists at: {path}")]
    ConfigAlreadyExists { path: PathBuf },

    #[error("Template file already exists at: {path}")]
    TemplateAlreadyExists { path: PathBuf },

    #[error("Failed to create directory: {path}")]
    DirectoryCreationFailed { path: PathBuf },

    #[error("Failed to write file: {path}")]
    FileWriteFailed { path: PathBuf },

    #[error("Config file not found at: {path}")]
    ConfigNotFound { path: PathBuf },

    #[error("Template file not found: {name}")]
    TemplateNotFound { name: String },

    #[error("Failed to parse config: {0}")]
    ParseError(String),

    #[error(
        "API key is missing. Set it in config file or GIT_COMMIT_GEN_API_KEY environment variable."
    )]
    ApiKeyMissing,
}

/// Handler errors
#[derive(Error, Debug)]
pub enum HandlerError {
    #[error("Failed to get current directory")]
    CurrentDirError,

    #[error("Invalid command: {0}")]
    InvalidCommand(String),

    #[error("Operation failed: {0}")]
    OperationFailed(String),
}

impl From<&str> for HandlerError {
    fn from(s: &str) -> Self {
        HandlerError::OperationFailed(s.to_string())
    }
}

impl From<String> for HandlerError {
    fn from(s: String) -> Self {
        HandlerError::OperationFailed(s)
    }
}
