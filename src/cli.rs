use clap::{Parser, Subcommand};

/// A CLI tool for automatically generating git commit messages
#[derive(Parser)]
#[command(name = "git-commit-gen")]
#[command(about = "Automatically generate git commit messages", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Generate a commit message based on staged changes
    #[command(alias = "gen")]
    Generate {
        /// Use a specific prompt template (overrides config)
        #[arg(short, long)]
        template: Option<String>,

        /// Interactively pick a template when --template is not provided
        #[arg(long)]
        pick_template: bool,

        /// Skip confirmation and directly commit with generated message
        #[arg(short, long)]
        yes: bool,

        /// Skip opening editor, use generated message directly (only works with --yes)
        #[arg(long)]
        no_edit: bool,
    },
    /// Configure AI settings and prompt templates
    Config {
        /// Set the AI API base URL
        #[arg(long)]
        base_url: Option<String>,

        /// Set the AI API key
        #[arg(long)]
        api_key: Option<String>,

        /// Set the model ID to use
        #[arg(long)]
        model_id: Option<String>,

        /// Set the default prompt template name
        #[arg(long)]
        default_template: Option<String>,
    },
}
