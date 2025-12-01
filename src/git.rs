use crate::errors::{AppError, GitError};
use std::path::PathBuf;
use std::process::Command;

/// Git repository operations wrapper
pub struct Git {
    work_dir: PathBuf,
}

impl Git {
    /// Create a new Git instance for the current directory
    pub fn new(work_dir: PathBuf) -> Self {
        Self { work_dir }
    }

    /// Check if git is available in the system
    pub fn is_available() -> bool {
        Command::new("git").arg("--version").output().is_ok()
    }

    /// Check if the current directory is a git repository
    pub fn is_repository(&self) -> bool {
        self.work_dir.join(".git").exists()
    }

    /// Get staged diff
    pub fn get_staged_diff(&self) -> Result<String, AppError> {
        let output = Command::new("git")
            .arg("diff")
            .arg("--staged")
            .arg("--no-color")
            .current_dir(&self.work_dir)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(GitError::CommandFailed {
                command: "git diff --staged".to_string(),
                stderr,
            }
            .into());
        }

        Ok(String::from_utf8(output.stdout)?)
    }

    /// Get list of staged files
    pub fn get_staged_files(&self) -> Result<Vec<String>, AppError> {
        let output = Command::new("git")
            .arg("diff")
            .arg("--staged")
            .arg("--name-only")
            .current_dir(&self.work_dir)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(GitError::CommandFailed {
                command: "git diff --staged --name-only".to_string(),
                stderr,
            }
            .into());
        }

        let files: Vec<String> = String::from_utf8(output.stdout)?
            .lines()
            .filter(|line| !line.is_empty())
            .map(|line| line.to_string())
            .collect();

        Ok(files)
    }

    /// Check if there are any staged changes
    pub fn has_staged_changes(&self) -> Result<bool, AppError> {
        let files = self.get_staged_files()?;
        Ok(!files.is_empty())
    }

    /// Get git status summary
    pub fn get_status(&self) -> Result<String, AppError> {
        let output = Command::new("git")
            .arg("status")
            .arg("--short")
            .current_dir(&self.work_dir)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(GitError::CommandFailed {
                command: "git status --short".to_string(),
                stderr,
            }
            .into());
        }

        Ok(String::from_utf8(output.stdout)?)
    }

    /// Commit with a message string
    pub fn commit_with_message(&self, message: &str) -> Result<(), AppError> {
        let output = Command::new("git")
            .arg("commit")
            .arg("-m")
            .arg(message)
            .current_dir(&self.work_dir)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(GitError::CommandFailed {
                command: "git commit -m".to_string(),
                stderr,
            }
            .into());
        }

        Ok(())
    }

    /// Commit with message but open editor with initial content
    pub fn commit_with_message_edit(&self, message: &str) -> Result<(), AppError> {
        let output = Command::new("git")
            .arg("commit")
            .arg("--edit")
            .arg("-m")
            .arg(message)
            .current_dir(&self.work_dir)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(GitError::CommandFailed {
                command: "git commit --edit -m".to_string(),
                stderr,
            }
            .into());
        }

        Ok(())
    }

    /// Get the current branch name
    pub fn get_current_branch(&self) -> Result<String, AppError> {
        let output = Command::new("git")
            .arg("branch")
            .arg("--show-current")
            .current_dir(&self.work_dir)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(GitError::CommandFailed {
                command: "git branch --show-current".to_string(),
                stderr,
            }
            .into());
        }

        let branch = String::from_utf8(output.stdout)?.trim().to_string();

        Ok(branch)
    }

    /// Get the repository root path
    pub fn get_repo_root(&self) -> Result<PathBuf, AppError> {
        let output = Command::new("git")
            .arg("rev-parse")
            .arg("--show-toplevel")
            .current_dir(&self.work_dir)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(GitError::CommandFailed {
                command: "git rev-parse --show-toplevel".to_string(),
                stderr,
            }
            .into());
        }

        let root = String::from_utf8(output.stdout)?.trim().to_string();

        Ok(PathBuf::from(root))
    }
}
