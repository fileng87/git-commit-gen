use crate::ai::AiClient;
use crate::config::Config;
use crate::errors::{AppError, ConfigError};
use crate::git::Git;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Core commit message generator
pub struct CommitGenerator {
    config: Config,
    git: Git,
}

impl CommitGenerator {
    /// Create a new CommitGenerator instance
    pub fn new(project_root: PathBuf, config: Config) -> Result<Self, AppError> {
        if !config.exists() {
            return Err(ConfigError::ConfigNotFound {
                path: config.config_path(),
            }.into());
        }

        let git = Git::new(project_root);
        
        Ok(Self {
            config,
            git,
        })
    }


    /// Generate commit message using LLM
    pub async fn generate_message(&self, template_name: Option<&str>) -> Result<String, AppError> {
        let (ai_config, templates_config) = self.config.load_config()?;
        
        // Determine which template to use
        let template_name = template_name.unwrap_or(&templates_config.default_template);
        let template = self.config.load_template(template_name)?;

        // Get git diff and file information
        let diff = self.git.get_staged_diff()?;
        let files = self.git.get_staged_files()?;

        // Build user prompt with diff and file list
        let user_prompt = self.build_user_prompt(&diff, &files);

        // Create AI client and call LLM API
        let ai_client = AiClient::new(ai_config)?;
        let message = ai_client.generate_commit_message(&template, &user_prompt).await?;

        Ok(message)
    }

    /// Build user prompt with diff and file information
    fn build_user_prompt(&self, diff: &str, files: &[String]) -> String {
        let mut prompt = String::new();
        
        prompt.push_str("## Staged Files\n\n");
        for file in files {
            prompt.push_str(&format!("- {}\n", file));
        }
        
        prompt.push_str("\n## Git Diff\n\n");
        prompt.push_str("```diff\n");
        prompt.push_str(diff);
        prompt.push_str("\n```\n");

        prompt
    }

    /// Open editor to edit commit message
    pub fn edit_message(&self, initial_message: &str) -> Result<String, AppError> {
        // Get git editor or fallback to default
        let editor = self.git.get_editor()?;
        
        // Create temporary file
        let temp_file = std::env::temp_dir().join(format!("git-commit-msg-{}.tmp", std::process::id()));
        fs::write(&temp_file, initial_message)?;

        // Open editor
        let status = Command::new(&editor)
            .arg(&temp_file)
            .status()?;

        if !status.success() {
            return Err(AppError::EditorError("Editor exited with non-zero status".to_string()));
        }

        // Read edited message
        let edited = fs::read_to_string(&temp_file)?;
        
        // Clean up temp file
        let _ = fs::remove_file(&temp_file);

        // Remove comments and empty lines
        let cleaned = self.clean_commit_message(&edited);

        if cleaned.trim().is_empty() {
            return Err(AppError::EditorError("Commit message is empty".to_string()));
        }

        Ok(cleaned)
    }

    /// Clean commit message (remove comments and empty lines)
    pub fn clean_commit_message(&self, message: &str) -> String {
        message
            .lines()
            .filter(|line| {
                let trimmed = line.trim();
                !trimmed.starts_with('#') && !trimmed.is_empty()
            })
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_string()
    }

    /// Commit with the generated message
    pub fn commit(&self, message: &str, no_edit: bool) -> Result<(), AppError> {
        if no_edit {
            self.git.commit_with_message(message)?;
        } else {
            // Write to temp file and use git commit -F
            let temp_file = std::env::temp_dir().join(format!("git-commit-msg-{}.tmp", std::process::id()));
            fs::write(&temp_file, message)?;
            self.git.commit_with_file(&temp_file)?;
            let _ = fs::remove_file(&temp_file);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_build_user_prompt() {
        use super::*;
        use tempfile::TempDir;
        
        let temp_dir = TempDir::new().unwrap();
        let temp_home = temp_dir.path().to_path_buf();
        let config = Config::new(temp_home);
        let git = Git::new(temp_dir.path().to_path_buf());
        let generator = CommitGenerator { config, git };
        
        let diff = "diff --git a/file.rs b/file.rs\n+new line";
        let files = vec!["file.rs".to_string(), "other.rs".to_string()];

        let prompt = generator.build_user_prompt(diff, &files);

        assert!(prompt.contains("## Staged Files"));
        assert!(prompt.contains("- file.rs"));
        assert!(prompt.contains("- other.rs"));
        assert!(prompt.contains("## Git Diff"));
        assert!(prompt.contains("```diff"));
        assert!(prompt.contains("diff --git"));
    }

    #[test]
    fn test_clean_commit_message() {
        use super::*;
        use tempfile::TempDir;
        
        let temp_dir = TempDir::new().unwrap();
        let temp_home = temp_dir.path().to_path_buf();
        let config = Config::new(temp_home);
        let git = Git::new(temp_dir.path().to_path_buf());
        let generator = CommitGenerator { config, git };
        
        let message_with_comments = "feat: add feature\n\n# This is a comment\n\nActual message\n\n# Another comment";
        let cleaned = generator.clean_commit_message(message_with_comments);
        
        // clean_commit_message removes all empty lines and comments
        assert_eq!(cleaned, "feat: add feature\nActual message");
        assert!(!cleaned.contains("#"));
    }

    #[test]
    fn test_clean_commit_message_empty_lines() {
        use super::*;
        use tempfile::TempDir;
        
        let temp_dir = TempDir::new().unwrap();
        let temp_home = temp_dir.path().to_path_buf();
        let config = Config::new(temp_home);
        let git = Git::new(temp_dir.path().to_path_buf());
        let generator = CommitGenerator { config, git };
        
        let message = "feat: add feature\n\n\n\n\nBody text";
        let cleaned = generator.clean_commit_message(message);
        
        // clean_commit_message removes all empty lines
        assert_eq!(cleaned, "feat: add feature\nBody text");
    }

    #[test]
    fn test_clean_commit_message_only_comments() {
        use super::*;
        use tempfile::TempDir;
        
        let temp_dir = TempDir::new().unwrap();
        let temp_home = temp_dir.path().to_path_buf();
        let config = Config::new(temp_home);
        let git = Git::new(temp_dir.path().to_path_buf());
        let generator = CommitGenerator { config, git };
        
        let message = "# Comment 1\n# Comment 2\n\n";
        let cleaned = generator.clean_commit_message(message);
        
        assert!(cleaned.is_empty());
    }

    #[test]
    fn test_clean_commit_message_preserves_content() {
        use super::*;
        use tempfile::TempDir;
        
        let temp_dir = TempDir::new().unwrap();
        let temp_home = temp_dir.path().to_path_buf();
        let config = Config::new(temp_home);
        let git = Git::new(temp_dir.path().to_path_buf());
        let generator = CommitGenerator { config, git };
        
        let message = "feat: add feature\n\nThis is the body\nWith multiple lines";
        let cleaned = generator.clean_commit_message(message);
        
        // clean_commit_message removes empty lines, so the double newline becomes single newline
        assert_eq!(cleaned, "feat: add feature\nThis is the body\nWith multiple lines");
    }
}

