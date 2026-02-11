use anyhow::Result;
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::config::Config;
use crate::tools::base::{Tool, ToolInvocation, ToolKind, ToolResult};

#[derive(Debug, Deserialize)]
struct WebFetchParams {
    url: String,
    #[serde(default = "default_timeout")]
    timeout: u64,
}

fn default_timeout() -> u64 {
    30
}

pub struct WebFetchTool {
    #[allow(dead_code)]
    config: Config,
}

impl WebFetchTool {
    pub fn new(config: Config) -> Self {
        Self { config }
    }
}

#[async_trait]
impl Tool for WebFetchTool {
    fn name(&self) -> &str {
        "web_fetch"
    }

    fn description(&self) -> &str {
        "Fetch content from a URL."
    }

    fn kind(&self) -> ToolKind {
        ToolKind::Network
    }

    fn schema(&self) -> Value {
        json!({"type":"object","properties":{"url":{"type":"string"},"timeout":{"type":"integer"}},"required":["url"]})
    }

    async fn execute(&self, invocation: ToolInvocation) -> Result<ToolResult> {
        let params: WebFetchParams = serde_json::from_value(invocation.params)?;
        if !(params.url.starts_with("http://") || params.url.starts_with("https://")) {
            return Ok(ToolResult::error_result(
                "Url must be http:// or https://".to_string(),
            ));
        }
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(params.timeout))
            .build()?;
        let response = client.get(&params.url).send().await?;
        let status = response.status();
        let text = response.text().await?;
        if !status.is_success() {
            return Ok(ToolResult::error_result(format!("HTTP {}", status)));
        }
        let mut body = text;
        if body.len() > 100 * 1024 {
            body.truncate(100 * 1024);
            body.push_str("\n... [content truncated]");
        }
        Ok(ToolResult::success_result(body))
    }
}
