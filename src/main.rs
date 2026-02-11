use clap::Parser;
use rust_coding_agent::cli::{Args, Cli};
use rust_coding_agent::config_loader::load_config;

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let config = match load_config(args.cwd.clone()) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Configuration Error: {}", e);
            std::process::exit(1);
        }
    };
    let errors = config.validate();
    if !errors.is_empty() {
        for err in errors {
            eprintln!("{}", err);
        }
        std::process::exit(1);
    }

    let mut cli = Cli::new(config);
    let result = if let Some(prompt) = args.prompt {
        cli.run_single(prompt).await
    } else {
        cli.run_interactive().await
    };
    if let Err(e) = result {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}
