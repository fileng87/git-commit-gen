use crate::cli::Commands;
use crate::config::Config;
use crate::core::CommitGenerator;
use crate::errors::{AppError, ConfigError, GitError, HandlerError};
use crate::git::Git;
use crate::utils;
use inquire::Select;
use std::env;
use std::fs;
use tokio::runtime::Runtime;
use toml::Value;

/// Command handler for git-commit-gen
pub struct Handler {
    project_root: std::path::PathBuf,
    home_dir: std::path::PathBuf,
}

impl Handler {
    /// Create a new Handler instance
    pub fn new() -> Result<Self, AppError> {
        let project_root = env::current_dir().map_err(|_| HandlerError::CurrentDirError)?;
        let home_dir = utils::get_home_dir()?;
        let config = Config::new(home_dir.clone());
        if !config.exists() {
            println!("Config not found; generating default config and templates...");
            config.generate_default_files(false)?;
        }
        Ok(Self {
            project_root,
            home_dir,
        })
    }

    /// Handle a command
    pub fn handle_command(&self, cmd: &Commands) -> Result<(), AppError> {
        match cmd {
            Commands::Generate {
                template,
                pick_template,
                yes,
                no_edit,
            } => self.handle_generate(template.as_deref(), *pick_template, *yes, *no_edit),
            Commands::Config {
                base_url,
                api_key,
                model_id,
                default_template,
            } => self.handle_config(base_url, api_key, model_id, default_template),
        }
    }

    /// Handle generate command
    fn handle_generate(
        &self,
        template: Option<&str>,
        pick_template: bool,
        yes: bool,
        no_edit: bool,
    ) -> Result<(), AppError> {
        if !Git::is_available() {
            return Err(GitError::NotAvailable.into());
        }
        let git = Git::new(self.project_root.clone());
        if !git.is_repository() {
            return Err(GitError::NotRepository.into());
        }
        if !git.has_staged_changes()? {
            return Err(GitError::NoStagedChanges.into());
        }

        let config = Config::new(self.home_dir.clone());
        let template_choice: Option<String> = match (template, pick_template) {
            (Some(name), _) => Some(name.to_string()),
            (None, true) => {
                let templates = config.list_templates()?;
                if templates.is_empty() {
                    return Err(ConfigError::TemplateNotFound {
                        name: "default".into(),
                    }
                    .into());
                }
                let selection = Select::new("Select a template:", templates)
                    .prompt()
                    .map_err(|e| {
                        HandlerError::OperationFailed(format!("Template prompt failed: {}", e))
                    })?;
                Some(selection)
            }
            _ => None,
        };
        let generator = CommitGenerator::new(self.project_root.clone(), config)?;

        let rt = Runtime::new().map_err(|e| {
            HandlerError::OperationFailed(format!("Failed to create runtime: {}", e))
        })?;
        let message = rt.block_on(generator.generate_message(template_choice.as_deref()))?;

        if yes {
            generator.commit(&message, no_edit)?;
            println!("✓ Commit created");
            return Ok(());
        }

        let prompt_message = format!(
            "Generated commit message:\n\n{}\n\nChoose an action:",
            message
        );
        let choices = vec!["Edit and commit", "Commit without editing", "Cancel"];

        let choice = Select::new(&prompt_message, choices)
            .prompt()
            .map_err(|e| HandlerError::OperationFailed(format!("Prompt failed: {}", e)))?;

        match choice {
            "Edit and commit" => {
                generator.commit(&message, false)?;
                println!("✓ Commit created");
            }
            "Commit without editing" => {
                generator.commit(&message, true)?;
                println!("✓ Commit created");
            }
            _ => println!("Cancelled."),
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
        if base_url.is_none()
            && api_key.is_none()
            && model_id.is_none()
            && default_template.is_none()
        {
            println!("No config fields provided; keeping current values.");
            return Ok(());
        }

        let config = Config::new(self.home_dir.clone());

        let config_path = config.config_path();
        let content =
            fs::read_to_string(&config_path).map_err(|_| ConfigError::ConfigNotFound {
                path: config_path.clone(),
            })?;

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

        fs::write(&config_path, updated).map_err(|_| ConfigError::FileWriteFailed {
            path: config_path.clone(),
        })?;

        println!("Updated config at {}", config_path.display());
        Ok(())
    }
}
