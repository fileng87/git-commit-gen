use clap::Parser;
use git_commit_gen::cli::Cli;
use git_commit_gen::handler::Handler;

fn main() {
    let cli = Cli::parse();

    let handler = match Handler::new() {
        Ok(h) => h,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    let result = match &cli.command {
        Some(cmd) => handler.handle_command(cmd),
        None => handler.handle_generate(
            cli.template.as_deref(),
            cli.pick_template,
            cli.yes,
            cli.no_edit,
        ),
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
