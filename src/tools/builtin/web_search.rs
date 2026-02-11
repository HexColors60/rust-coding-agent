use anyhow::Result;
use async_trait::async_trait;
use regex::Regex;
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use serde::Deserialize;
use serde_json::{Value, json};
use std::cmp::min;

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
        let max_results = min(params.max_results.max(1), 20);
        let query_url = format!(
            "https://html.duckduckgo.com/html/?q={}",
            urlencoding::encode(&params.query)
        );

        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static("rust-coding-agent/0.1 (+https://example.local)"),
        );
        let client = reqwest::Client::builder().default_headers(headers).build()?;
        let response = client.get(query_url).send().await?;
        if !response.status().is_success() {
            return Ok(ToolResult::error_result(format!(
                "Search failed: HTTP {}",
                response.status()
            )));
        }
        let html = response.text().await?;
        let item_re = Regex::new(
            r#"<a[^>]*class="[^"]*result__a[^"]*"[^>]*href="(?P<href>[^"]+)"[^>]*>(?P<title>.*?)</a>"#,
        )?;
        let snippet_re = Regex::new(
            r#"<a[^>]*class="[^"]*result__snippet[^"]*"[^>]*>(?P<snippet>.*?)</a>|<div[^>]*class="[^"]*result__snippet[^"]*"[^>]*>(?P<snippet2>.*?)</div>"#,
        )?;
        let strip_tags = Regex::new(r"<[^>]+>")?;

        let mut results = Vec::new();
        for caps in item_re.captures_iter(&html) {
            let href = caps
                .name("href")
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();
            let title_raw = caps.name("title").map(|m| m.as_str()).unwrap_or_default();
            let title = strip_tags.replace_all(title_raw, "").to_string();
            results.push((title, href));
            if results.len() >= max_results {
                break;
            }
        }

        if results.is_empty() {
            return Ok(ToolResult::success_result(format!(
                "No results found for: {}",
                params.query
            )));
        }

        let snippets: Vec<String> = snippet_re
            .captures_iter(&html)
            .map(|c| {
                let raw = c
                    .name("snippet")
                    .or_else(|| c.name("snippet2"))
                    .map(|m| m.as_str())
                    .unwrap_or_default();
                strip_tags.replace_all(raw, "").to_string()
            })
            .collect();

        let mut output_lines = vec![format!("Search results for: {}", params.query)];
        for (i, (title, href)) in results.iter().enumerate() {
            output_lines.push(format!("{}. Title: {}", i + 1, title));
            output_lines.push(format!("   URL: {}", href));
            if let Some(snippet) = snippets.get(i) {
                if !snippet.trim().is_empty() {
                    output_lines.push(format!("   Snippet: {}", snippet.trim()));
                }
            }
            output_lines.push(String::new());
        }
        Ok(ToolResult::success_result(output_lines.join("\n")))
    }
}
