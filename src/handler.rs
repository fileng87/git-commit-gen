use crate::cli::Commands;
use crate::config::Config;
use crate::core::CommitGenerator;
use crate::errors::{AppError, ConfigError, GitError, HandlerError};
use crate::git::Git;
use crate::utils;
use inquire::Select;
use std::fs;
use std::env;
use toml::Value;

/// Command handler for git-commit-gen
pub struct Handler {
    project_root: std::path::PathBuf,
}

impl Handler {
    /// Create a new Handler instance
    pub fn new() -> Result<Self, AppError> {
        let project_root = env::current_dir()
            .map_err(|_| HandlerError::CurrentDirError)?;
        Ok(Self { project_root })
    }

    /// Handle a command
    pub fn handle_command(&self, cmd: &Commands) -> Result<(), AppError> {
        match cmd {
            Commands::Init { force } => self.handle_init(*force),
            Commands::Generate { template, yes, no_edit } => {
                self.handle_generate(template.as_deref(), *yes, *no_edit)
            },
            Commands::Config { base_url, api_key, model_id, default_template } => 
                self.handle_config(base_url, api_key, model_id, default_template),
        }
    }

    /// Handle init command
    fn handle_init(&self, force: bool) -> Result<(), AppError> {
        let home_dir = utils::get_home_dir()?;
        let config = Config::new(home_dir);
        
        if force {
            config.remove_existing_files()?;
        }
        
        match config.generate_default_files(force) {
            Ok(()) => Ok(()),
            Err(e) => {
                eprintln!("✗ Failed to create files: {}", e);
                if !force {
                    eprintln!("  Use --force to overwrite existing files");
                }
                Err(e)
            }
        }
    }

    /// Handle generate command
    fn handle_generate(&self, template: Option<&str>, yes: bool, no_edit: bool) -> Result<(), AppError> {
        // Check if git is available
        if !Git::is_available() {
            return Err(GitError::NotAvailable.into());
        }

        // Check if we're in a git repository
        let git = Git::new(self.project_root.clone());
        if !git.is_repository() {
            return Err(GitError::NotRepository.into());
        }

        // Check if there are staged changes
        if !git.has_staged_changes()? {
            return Err(GitError::NoStagedChanges.into());
        }

        let home_dir = utils::get_home_dir()?;
        let config = Config::new(home_dir);
        let generator = CommitGenerator::new(self.project_root.clone(), config)?;
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| HandlerError::OperationFailed(format!("Failed to create runtime: {}", e)))?;
        let message = rt.block_on(generator.generate_message(template))?;

        if yes {
            let final_message = if no_edit {
                message
            } else {
                generator.edit_message(&message)?
            };
            generator.commit(&final_message, no_edit)?;
            println!("✓ Commit created");
            return Ok(());
        }

        let prompt_message = format!(
            "Generated commit message:\n\n{}\n\nChoose an action:",
            message
        );
        let choices = vec![
            "Edit and commit",
            "Commit without editing",
            "Cancel",
        ];

        let choice = Select::new(&prompt_message, choices)
            .prompt()
            .map_err(|e| HandlerError::OperationFailed(format!("Prompt failed: {}", e)))?;

        match choice {
            "Edit and commit" => {
                let final_message = generator.edit_message(&message)?;
                generator.commit(&final_message, false)?;
                println!("✓ Commit created");
            }
            "Commit without editing" => {
                generator.commit(&message, true)?;
                println!("✓ Commit created");
            }
            _ => {
                println!("Cancelled.");
            }
        }

        Ok(())
    }

    /// Handle config update command
    fn handle_config(
        &self,
        base_url: &Option<String>,
        api_key: &Option<String>,
        model_id: &Option<String>,
        default_template: &Option<String>,
    ) -> Result<(), AppError> {
        if base_url.is_none() && api_key.is_none() && model_id.is_none() && default_template.is_none() {
            println!("No config fields provided; keeping current values.");
            return Ok(());
        }

        let home_dir = utils::get_home_dir()?;
        let config = Config::new(home_dir);
        if !config.exists() {
            return Err(ConfigError::ConfigNotFound { path: config.config_path() }.into());
        }

        let config_path = config.config_path();
        let content = fs::read_to_string(&config_path)
            .map_err(|_| ConfigError::ConfigNotFound { path: config_path.clone() })?;

        let mut value: Value = toml::from_str(&content)
            .map_err(|e| ConfigError::ParseError(format!("Failed to parse config: {}", e)))?;

        if let Some(v) = base_url {
            value["ai"]["base_url"] = Value::String(v.clone());
        }
        if let Some(v) = api_key {
            value["ai"]["api_key"] = Value::String(v.clone());
        }
        if let Some(v) = model_id {
            value["ai"]["model_id"] = Value::String(v.clone());
        }
        if let Some(v) = default_template {
            value["templates"]["default_template"] = Value::String(v.clone());
        }

        let updated = toml::to_string_pretty(&value)
            .map_err(|e| ConfigError::ParseError(format!("Failed to serialize config: {}", e)))?;

        fs::write(&config_path, updated)
            .map_err(|_| ConfigError::FileWriteFailed { path: config_path.clone() })?;

        println!("Updated config at {}", config_path.display());
        Ok(())
    }
}
