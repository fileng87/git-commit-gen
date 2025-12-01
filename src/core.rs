use crate::ai::AiClient;
use crate::config::Config;
use crate::errors::{AppError, ConfigError};
use crate::git::Git;
use std::path::PathBuf;

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
            }
            .into());
        }

        let git = Git::new(project_root);

        Ok(Self { config, git })
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
        let message = ai_client
            .generate_commit_message(&template, &user_prompt)
            .await?;

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

    /// Commit with the generated message
    pub fn commit(&self, message: &str, no_edit: bool) -> Result<(), AppError> {
        if no_edit {
            self.git.commit_with_message(message)?;
        } else {
            self.git.commit_with_message_edit(message)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn build_user_prompt_includes_files_and_diff() {
        let temp_dir = TempDir::new().unwrap();
        let config = Config::new(temp_dir.path().to_path_buf());
        let git = Git::new(temp_dir.path().to_path_buf());
        let generator = CommitGenerator { config, git };

        let diff = "diff --git a/file.rs b/file.rs\n+hello";
        let files = vec!["file.rs".to_string(), "other.rs".to_string()];

        let prompt = generator.build_user_prompt(diff, &files);

        assert!(prompt.contains("## Staged Files"));
        assert!(prompt.contains("- file.rs"));
        assert!(prompt.contains("- other.rs"));
        assert!(prompt.contains("## Git Diff"));
        assert!(prompt.contains("```diff"));
        assert!(prompt.contains("diff --git"));
        assert!(prompt.contains("+hello"));
    }
}
