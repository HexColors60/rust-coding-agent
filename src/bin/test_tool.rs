use std::env;

use anyhow::Result;
use rust_coding_agent::config_loader::load_config;
use rust_coding_agent::registry::create_default_registry;

#[tokio::main]
async fn main() -> Result<()> {
    let tool_name = env::var("AI_AGENT_TOOL_NAME").unwrap_or_else(|_| "list_dir".to_string());
    let params_raw = env::var("AI_AGENT_TOOL_PARAMS").unwrap_or_else(|_| "{}".to_string());
    let params: serde_json::Value = serde_json::from_str(&params_raw)?;
    let config = load_config(None)?;
    let registry = create_default_registry(&config);
    let result = registry
        .invoke(&tool_name, params, &config.cwd, None)
        .await?;
    println!("success={}", result.success);
    println!("output={}", result.output);
    if let Some(err) = result.error {
        println!("error={}", err);
    }
    Ok(())
}
