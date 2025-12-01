use git_commit_gen::config::Config;
use git_commit_gen::core::CommitGenerator;
use git_commit_gen::git::Git;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;
use toml::value::Table;
use toml::Value;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const MOCK_API_KEY: &str = "test-api-key";

/// Ensure a table exists on the given key in the TOML document
fn get_or_create_table<'a>(
    doc: &'a mut Value,
    key: &str,
) -> Result<&'a mut Table, Box<dyn std::error::Error>> {
    let root = doc
        .as_table_mut()
        .ok_or_else(|| {
            let err: Box<dyn std::error::Error> = "config root is not a table".into();
            err
        })?;
    Ok(root
        .entry(key.to_string())
        .or_insert_with(|| Value::Table(Table::new()))
        .as_table_mut()
        .ok_or_else(|| {
            let err: Box<dyn std::error::Error> = format!("{key} is not a table").into();
            err
        })?)
}

/// Setup a test git repository with staged changes
fn setup_test_repo() -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path().to_path_buf();

    // Initialize git repository
    Command::new("git")
        .arg("init")
        .current_dir(&repo_path)
        .output()
        .unwrap();

    // Configure git user (required for commits)
    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(&repo_path)
        .output()
        .unwrap();

    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(&repo_path)
        .output()
        .unwrap();

    // Create a test file
    let test_file = repo_path.join("test.rs");
    fs::write(&test_file, "fn main() {\n    println!(\"Hello\");\n}").unwrap();

    // Stage the file
    Command::new("git")
        .args(["add", "test.rs"])
        .current_dir(&repo_path)
        .output()
        .unwrap();

    (temp_dir, repo_path)
}

/// Setup default config files (no env overrides; mock 覆寫再補)
fn setup_config(repo_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::new(repo_path.clone());
    config.generate_default_files(true)?;
    Ok(())
}

/// Update config to use mock server URL and test API key
fn update_config_for_mock(
    repo_path: &PathBuf,
    mock_url: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::new(repo_path.clone());
    let config_path = config.config_path();
    let config_content = fs::read_to_string(&config_path)?;
    let mut config_value: Value = toml::from_str(&config_content)?;

    let ai_table = get_or_create_table(&mut config_value, "ai")?;
    ai_table.insert("base_url".into(), Value::String(mock_url.to_string()));

    let api_key_empty = ai_table
        .get("api_key")
        .and_then(Value::as_str)
        .map(|v| v.is_empty())
        .unwrap_or(true);
    if api_key_empty {
        ai_table.insert("api_key".into(), Value::String(MOCK_API_KEY.to_string()));
    }

    fs::write(&config_path, toml::to_string(&config_value)?)?;
    Ok(())
}

#[tokio::test]
async fn test_generate_commit_message_with_mock_api() {
    // Setup test repository
    let (_temp_dir, repo_path) = setup_test_repo();

    // Setup config
    setup_config(&repo_path).unwrap();

    // Setup mock server
    let mock_server = MockServer::start().await;

    // Mock the LLM API response
    let mock_response = serde_json::json!({
        "choices": [{
            "message": {
                "role": "assistant",
                "content": "feat: add test file with hello world function"
            }
        }]
    });

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .and(header("Authorization", format!("Bearer {}", MOCK_API_KEY)))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_response))
        .mount(&mock_server)
        .await;

    // Update config to use mock server URL
    let mock_url = format!("http://127.0.0.1:{}", mock_server.address().port());
    update_config_for_mock(&repo_path, &mock_url).unwrap();

    // Create generator and test
    let generator = CommitGenerator::new(repo_path.clone()).unwrap();
    let message = generator.generate_message(None).await.unwrap();

    assert!(message.contains("feat"));
    assert!(message.contains("test"));
}

#[tokio::test]
async fn test_generate_commit_message_with_conventional_template() {
    let (_temp_dir, repo_path) = setup_test_repo();
    setup_config(&repo_path).unwrap();

    let mock_server = MockServer::start().await;

    let mock_response = serde_json::json!({
        "choices": [{
            "message": {
                "role": "assistant",
                "content": "feat(test): add test file"
            }
        }]
    });

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_response))
        .mount(&mock_server)
        .await;

    let mock_url = format!("http://127.0.0.1:{}", mock_server.address().port());
    update_config_for_mock(&repo_path, &mock_url).unwrap();

    let generator = CommitGenerator::new(repo_path.clone()).unwrap();
    let message = generator.generate_message(Some("conventional")).await.unwrap();

    assert!(message.contains("feat"));
}

#[tokio::test]
#[ignore]
async fn test_generate_commit_message_with_real_api() {
    // 需要設定 API_KEY（及可選 BASE_URL/MODEL_ID）才能執行，故標記忽略
    dotenvy::dotenv().ok();

    let api_key = std::env::var("API_KEY").expect("API_KEY must be set for real API test");
    let base_url = std::env::var("BASE_URL").unwrap_or_else(|_| "https://api.openai.com/v1".to_string());
    let model_id = std::env::var("MODEL_ID").unwrap_or_else(|_| "gpt-4o-mini".to_string());

    println!("\n=== Real API Test Configuration ===");
    println!("Base URL: {}", base_url);
    println!("Model ID: {}", model_id);
    println!("===================================\n");

    let (_temp_dir, repo_path) = setup_test_repo();
    setup_config(&repo_path).unwrap();

    // 覆寫配置為實際環境值
    let config = Config::new(repo_path.clone());
    let config_path = config.config_path();
    let config_content = fs::read_to_string(&config_path).unwrap();
    let mut config_value: Value = toml::from_str(&config_content).unwrap();
    {
        let ai_table = get_or_create_table(&mut config_value, "ai").unwrap();
        ai_table.insert("base_url".into(), Value::String(base_url));
        ai_table.insert("model_id".into(), Value::String(model_id));
        ai_table.insert("api_key".into(), Value::String(api_key));
    }
    fs::write(&config_path, toml::to_string(&config_value).unwrap()).unwrap();

    let generator = CommitGenerator::new(repo_path.clone()).unwrap();
    let message = generator.generate_message(None).await.unwrap();

    println!("=== Generated Commit Message (Raw) ===");
    println!("{}", message);
    println!("=====================================\n");

    let cleaned = generator.clean_commit_message(&message);
    
    println!("=== Generated Commit Message (Cleaned) ===");
    println!("{}", cleaned);
    println!("==========================================\n");

    assert!(!cleaned.trim().is_empty());
}

#[test]
fn test_init_command_creates_config_and_templates() {
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path().to_path_buf();

    let config = Config::new(repo_path.clone());
    config.generate_default_files(false).unwrap();

    // Check config file exists
    assert!(config.config_path().exists());
    let config_content = fs::read_to_string(config.config_path()).unwrap();
    assert!(config_content.contains("[ai]"));
    assert!(config_content.contains("[templates]"));

    // Check template files exist
    assert!(config.template_path("default").exists());
    assert!(config.template_path("conventional").exists());
}

#[test]
fn test_git_operations() {
    let (_temp_dir, repo_path) = setup_test_repo();
    let git = Git::new(repo_path.clone());

    // Test repository check
    assert!(git.is_repository());

    // Test staged files
    let files = git.get_staged_files().unwrap();
    assert!(!files.is_empty());
    assert!(files.contains(&"test.rs".to_string()));

    // Test staged diff
    let diff = git.get_staged_diff().unwrap();
    assert!(diff.contains("+"));
    assert!(diff.contains("Hello"));
}

#[test]
fn test_config_load_default() {
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path().to_path_buf();

    let config = Config::new(repo_path.clone());
    config.generate_default_files(false).unwrap();

    setup_config(&repo_path).unwrap();

    // Inject API key into config for loading
    let config_path = config.config_path();
    let config_content = fs::read_to_string(&config_path).unwrap();
    let mut config_value: Value = toml::from_str(&config_content).unwrap();
    {
        let ai_table = get_or_create_table(&mut config_value, "ai").unwrap();
        ai_table.insert("api_key".into(), Value::String(MOCK_API_KEY.to_string()));
    }
    fs::write(&config_path, toml::to_string(&config_value).unwrap()).unwrap();

    // Load and verify config
    let (ai_config, templates_config) = config.load_config().unwrap();
    assert!(ai_config.has_api_key());

    // Verify template config
    assert!(!templates_config.default_template.is_empty());
}

#[tokio::test]
async fn test_full_workflow_with_mock() {
    let (_temp_dir, repo_path) = setup_test_repo();
    setup_config(&repo_path).unwrap();

    let mock_server = MockServer::start().await;

    let mock_response = serde_json::json!({
        "choices": [{
            "message": {
                "role": "assistant",
                "content": "feat: add test file\n\nAdd a simple hello world function in Rust"
            }
        }]
    });

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_response))
        .mount(&mock_server)
        .await;

    let mock_url = format!("http://127.0.0.1:{}", mock_server.address().port());
    update_config_for_mock(&repo_path, &mock_url).unwrap();

    // Test full workflow
    let generator = CommitGenerator::new(repo_path.clone()).unwrap();
    
    // Generate message
    let message = generator.generate_message(None).await.unwrap();
    assert!(!message.is_empty());
    assert!(message.contains("feat"));

    // Test clean commit message
    let cleaned = generator.clean_commit_message(&message);
    assert!(!cleaned.is_empty());
}

