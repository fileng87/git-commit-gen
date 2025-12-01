use crate::ai::AiConfig;
use crate::errors::{AppError, ConfigError};
use crate::templates;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

// Embed the default config template
const DEFAULT_CONFIG: &str = include_str!("../assets/config.toml");

/// Templates configuration from config file
#[derive(Debug, Deserialize)]
pub struct TemplatesConfig {
    pub default_template: String,
}

/// Full configuration structure (for deserialization)
#[derive(Debug, Deserialize)]
struct ConfigFile {
    ai: AiConfig,
    templates: TemplatesConfig,
}

/// Configuration manager for git-commit-gen
pub struct Config {
    project_root: PathBuf,
}

impl Config {
    /// Create a new Config instance for the given project root
    pub fn new(project_root: PathBuf) -> Self {
        Self { project_root }
    }

    /// Get the config directory path
    pub fn config_dir(&self) -> PathBuf {
        self.project_root.join(".git-commit-gen")
    }

    /// Get the templates directory path
    pub fn templates_dir(&self) -> PathBuf {
        self.config_dir().join("templates")
    }

    /// Get the config file path
    pub fn config_path(&self) -> PathBuf {
        self.config_dir().join("config.toml")
    }

    /// Get the template file path
    pub fn template_path(&self, template_name: &str) -> PathBuf {
        self.templates_dir().join(format!("{}.md", template_name))
    }

    /// Check if config exists
    pub fn exists(&self) -> bool {
        self.config_path().exists()
    }

    /// Generate default config and template files
    pub fn generate_default_files(&self, force: bool) -> Result<(), AppError> {
        let config_dir = self.config_dir();
        let templates_dir = self.templates_dir();
        
        // Create directories if they don't exist
        if !config_dir.exists() {
            fs::create_dir_all(&config_dir)
                .map_err(|_| ConfigError::DirectoryCreationFailed { path: config_dir.clone() })?;
        }
        if !templates_dir.exists() {
            fs::create_dir_all(&templates_dir)
                .map_err(|_| ConfigError::DirectoryCreationFailed { path: templates_dir.clone() })?;
        }
        
        // Generate config file
        let config_path = self.config_path();
        if config_path.exists() && !force {
            return Err(ConfigError::ConfigAlreadyExists { path: config_path }.into());
        }
        fs::write(&config_path, DEFAULT_CONFIG)
            .map_err(|_| ConfigError::FileWriteFailed { path: config_path.clone() })?;
        println!("✓ Created config file at: {}", config_path.display());
        
        // Generate template files
        let default_template_path = self.template_path("default");
        if default_template_path.exists() && !force {
            return Err(ConfigError::TemplateAlreadyExists { path: default_template_path }.into());
        }
        if let Some(content) = templates::get_builtin_template("default") {
            fs::write(&default_template_path, content)
                .map_err(|_| ConfigError::FileWriteFailed { path: default_template_path.clone() })?;
            println!("✓ Created template file at: {}", default_template_path.display());
        }
        
        let conventional_template_path = self.template_path("conventional");
        if conventional_template_path.exists() && !force {
            return Err(ConfigError::TemplateAlreadyExists { path: conventional_template_path }.into());
        }
        if let Some(content) = templates::get_builtin_template("conventional") {
            fs::write(&conventional_template_path, content)
                .map_err(|_| ConfigError::FileWriteFailed { path: conventional_template_path.clone() })?;
            println!("✓ Created template file at: {}", conventional_template_path.display());
        }
        
        Ok(())
    }

    /// Remove existing config and template files (for force overwrite)
    pub fn remove_existing_files(&self) -> Result<(), AppError> {
        let config_path = self.config_path();
        if config_path.exists() {
            fs::remove_file(&config_path)?;
        }
        
        let default_template = self.template_path("default");
        if default_template.exists() {
            fs::remove_file(&default_template)?;
        }
        
        let conventional_template = self.template_path("conventional");
        if conventional_template.exists() {
            fs::remove_file(&conventional_template)?;
        }
        
        Ok(())
    }

    /// Load configuration from file
    pub fn load_config(&self) -> Result<(AiConfig, TemplatesConfig), AppError> {
        let config_path = self.config_path();
        let config_content = fs::read_to_string(&config_path)
            .map_err(|_| ConfigError::ConfigNotFound { path: config_path.clone() })?;

        let config: ConfigFile = toml::from_str(&config_content)
            .map_err(|e| ConfigError::ParseError(format!("Failed to parse config: {}", e)))?;

        let ai_config = config.ai.finalize();

        if !ai_config.has_api_key() {
            return Err(ConfigError::ApiKeyMissing.into());
        }

        Ok((ai_config, config.templates))
    }

    /// Load template content
    pub fn load_template(&self, template_name: &str) -> Result<String, AppError> {
        let template_path = self.template_path(template_name);
        
        if !template_path.exists() {
            // Try built-in template
            if let Some(content) = templates::get_builtin_template(template_name) {
                return Ok(content.to_string());
            }
            return Err(ConfigError::TemplateNotFound {
                name: template_name.to_string(),
            }.into());
        }

        fs::read_to_string(&template_path)
            .map_err(|_| ConfigError::TemplateNotFound {
                name: template_name.to_string(),
            })
            .map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_config_dir() {
        let project_root = PathBuf::from("/test/project");
        let config = Config::new(project_root);
        assert_eq!(config.config_dir(), PathBuf::from("/test/project/.git-commit-gen"));
    }

    #[test]
    fn test_templates_dir() {
        let project_root = PathBuf::from("/test/project");
        let config = Config::new(project_root);
        assert_eq!(config.templates_dir(), PathBuf::from("/test/project/.git-commit-gen/templates"));
    }

    #[test]
    fn test_config_path() {
        let project_root = PathBuf::from("/test/project");
        let config = Config::new(project_root);
        assert_eq!(config.config_path(), PathBuf::from("/test/project/.git-commit-gen/config.toml"));
    }

    #[test]
    fn test_template_path() {
        let project_root = PathBuf::from("/test/project");
        let config = Config::new(project_root);
        assert_eq!(config.template_path("default"), PathBuf::from("/test/project/.git-commit-gen/templates/default.md"));
    }

    #[test]
    fn test_generate_default_files() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path().to_path_buf();
        let config = Config::new(project_root);
        
        // Generate files
        config.generate_default_files(false).unwrap();
        
        // Check config file exists
        let config_path = config.config_path();
        assert!(config_path.exists());
        let config_content = fs::read_to_string(&config_path).unwrap();
        assert!(config_content.contains("[ai]"));
        assert!(config_content.contains("[templates]"));
        
        // Check template files exist
        let default_template = config.template_path("default");
        assert!(default_template.exists());
        let default_content = fs::read_to_string(&default_template).unwrap();
        assert!(default_content.contains("Simple Commit Message Generator"));
        
        let conventional_template = config.template_path("conventional");
        assert!(conventional_template.exists());
        let conventional_content = fs::read_to_string(&conventional_template).unwrap();
        assert!(conventional_content.contains("Conventional Commits Generator"));
    }

    #[test]
    fn test_generate_default_files_already_exists() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path().to_path_buf();
        let config = Config::new(project_root);
        
        // Generate files first time
        config.generate_default_files(false).unwrap();
        
        // Try to generate again without force - should fail
        let result = config.generate_default_files(false);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[test]
    fn test_generate_default_files_force_overwrite() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path().to_path_buf();
        let config = Config::new(project_root);
        
        // Generate files first time
        config.generate_default_files(false).unwrap();
        
        // Modify config file
        let config_path = config.config_path();
        fs::write(&config_path, "modified content").unwrap();
        
        // Generate again with force - should overwrite
        config.generate_default_files(true).unwrap();
        
        // Check that content was overwritten
        let config_content = fs::read_to_string(&config_path).unwrap();
        assert!(config_content.contains("[ai]"));
        assert!(!config_content.contains("modified content"));
    }

    #[test]
    fn test_config_exists() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path().to_path_buf();
        let config = Config::new(project_root);
        
        // Initially doesn't exist
        assert!(!config.exists());
        
        // Generate files
        config.generate_default_files(false).unwrap();
        
        // Now exists
        assert!(config.exists());
    }

    #[test]
    fn test_load_config() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path().to_path_buf();
        let config = Config::new(project_root.clone());
        
        // Generate default files
        config.generate_default_files(false).unwrap();
        
        // Modify config file to include API key
        let config_path = config.config_path();
        let mut config_content = fs::read_to_string(&config_path).unwrap();
        config_content = config_content.replace("api_key = \"\"", "api_key = \"test-api-key\"");
        fs::write(&config_path, config_content).unwrap();
        
        // Load config
        let (ai_config, templates_config) = config.load_config().unwrap();
        
        assert_eq!(ai_config.model_id, "gpt-4");
        assert_eq!(templates_config.default_template, "default");
        assert_eq!(ai_config.api_key, "test-api-key");
    }

    #[test]
    fn test_load_config_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path().to_path_buf();
        let config = Config::new(project_root);
        
        // Try to load config that doesn't exist
        let result = config.load_config();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            AppError::Config(ConfigError::ConfigNotFound { .. })
        ));
    }

    #[test]
    fn test_load_template_from_file() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path().to_path_buf();
        let config = Config::new(project_root);
        
        // Generate default files
        config.generate_default_files(false).unwrap();
        
        // Load template
        let template = config.load_template("default").unwrap();
        assert!(template.contains("Simple Commit Message Generator"));
    }

    #[test]
    fn test_load_template_builtin() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path().to_path_buf();
        let config = Config::new(project_root);
        
        // Load built-in template (file doesn't exist, should use built-in)
        let template = config.load_template("default").unwrap();
        assert!(template.contains("Simple Commit Message Generator"));
    }

    #[test]
    fn test_load_template_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path().to_path_buf();
        let config = Config::new(project_root);
        
        // Try to load non-existent template
        let result = config.load_template("nonexistent");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            AppError::Config(ConfigError::TemplateNotFound { .. })
        ));
    }
}
