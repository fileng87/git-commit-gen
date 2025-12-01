use crate::errors::{AppError, ConfigError};
use std::fs;
use std::path::PathBuf;

// Embed default templates
pub const DEFAULT_TEMPLATE: &str = include_str!("../assets/templates/default.md");
pub const CONVENTIONAL_TEMPLATE: &str = include_str!("../assets/templates/conventional.md");

/// Get a built-in template by name
pub fn get_builtin_template(name: &str) -> Option<&'static str> {
    match name {
        "default" => Some(DEFAULT_TEMPLATE),
        "conventional" => Some(CONVENTIONAL_TEMPLATE),
        _ => None,
    }
}

/// List all available built-in template names
pub fn list_builtin_templates() -> Vec<&'static str> {
    vec!["default", "conventional"]
}

/// Load template content from file or built-in templates
pub fn load_template(template_path: &PathBuf, template_name: &str) -> Result<String, AppError> {
    if !template_path.exists() {
        // Try built-in template
        if let Some(content) = get_builtin_template(template_name) {
            return Ok(content.to_string());
        }
        return Err(ConfigError::TemplateNotFound {
            name: template_name.to_string(),
        }
        .into());
    }

    fs::read_to_string(template_path)
        .map_err(|_| ConfigError::TemplateNotFound {
            name: template_name.to_string(),
        })
        .map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_builtin_template_default() {
        let template = get_builtin_template("default");
        assert!(template.is_some());
        assert!(
            template
                .unwrap()
                .contains("Simple Commit Message Generator")
        );
    }

    #[test]
    fn test_get_builtin_template_conventional() {
        let template = get_builtin_template("conventional");
        assert!(template.is_some());
        assert!(template.unwrap().contains("Conventional Commits Generator"));
    }

    #[test]
    fn test_get_builtin_template_invalid() {
        let template = get_builtin_template("invalid");
        assert!(template.is_none());
    }

    #[test]
    fn test_list_builtin_templates() {
        let templates = list_builtin_templates();
        assert_eq!(templates.len(), 2);
        assert!(templates.contains(&"default"));
        assert!(templates.contains(&"conventional"));
    }

    #[test]
    fn test_templates_not_empty() {
        let default = get_builtin_template("default").unwrap();
        let conventional = get_builtin_template("conventional").unwrap();

        assert!(!default.is_empty());
        assert!(!conventional.is_empty());
    }
}
