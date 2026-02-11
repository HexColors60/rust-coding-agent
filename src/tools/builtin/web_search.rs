use anyhow::Result;
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::config::Config;
use crate::tools::base::{Tool, ToolInvocation, ToolKind, ToolResult};

#[derive(Debug, Deserialize)]
struct WebSearchParams {
    query: String,
    #[serde(default = "default_max_results")]
    max_results: usize,
}

fn default_max_results() -> usize {
    10
}

pub struct WebSearchTool {
    #[allow(dead_code)]
    config: Config,
}

impl WebSearchTool {
    pub fn new(config: Config) -> Self {
        Self { config }
    }
}

#[async_trait]
impl Tool for WebSearchTool {
    fn name(&self) -> &str {
        "web_search"
    }

    fn description(&self) -> &str {
        "Search the web for information."
    }

    fn kind(&self) -> ToolKind {
        ToolKind::Network
    }

    fn schema(&self) -> Value {
        json!({"type":"object","properties":{"query":{"type":"string"},"max_results":{"type":"integer"}},"required":["query"]})
    }

    async fn execute(&self, invocation: ToolInvocation) -> Result<ToolResult> {
        let params: WebSearchParams = serde_json::from_value(invocation.params)?;
        Ok(ToolResult::success_result(format!(
            "web_search placeholder for query='{}' (max_results={})",
            params.query, params.max_results
        )))
    }
}
