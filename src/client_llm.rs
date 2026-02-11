use anyhow::{Result, anyhow};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue};
use serde_json::Value;
use tokio::time::{Duration, sleep};

use crate::client_response::{StreamEvent, StreamEventType, parse_tool_call_arguments};
use crate::config::Config;

pub struct LLMClient {
    client: reqwest::Client,
    max_retries: usize,
    config: Config,
}

impl LLMClient {
    pub fn new(config: Config) -> Self {
        Self {
            client: reqwest::Client::new(),
            max_retries: 3,
            config,
        }
    }

    fn build_tools(&self, tools: &[Value]) -> Vec<Value> {
        tools
            .iter()
            .map(|tool| {
                serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": tool.get("name").and_then(|v| v.as_str()).unwrap_or_default(),
                        "description": tool.get("description").and_then(|v| v.as_str()).unwrap_or_default(),
                        "parameters": tool.get("parameters").cloned().unwrap_or_else(|| serde_json::json!({"type":"object","properties":{}}))
                    }
                })
            })
            .collect()
    }

    pub async fn chat(&self, messages: Vec<Value>, tools: Vec<Value>) -> Result<Vec<StreamEvent>> {
        if self.config.api_key.is_none() {
            let fallback = messages
                .iter()
                .rev()
                .find(|m| m.get("role").and_then(|x| x.as_str()) == Some("user"))
                .and_then(|m| m.get("content").and_then(|x| x.as_str()))
                .unwrap_or_default();
            return Ok(vec![StreamEvent {
                event_type: StreamEventType::TextComplete,
                data: serde_json::json!({"content": format!("(offline fallback) {}", fallback)}),
            }]);
        }

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!(
                "Bearer {}",
                self.config.api_key.clone().unwrap_or_default()
            ))?,
        );
        let payload = serde_json::json!({
            "model": self.config.model_name,
            "messages": messages,
            "stream": false,
            "temperature": self.config.temperature,
            "tools": if tools.is_empty() { Value::Null } else { Value::Array(self.build_tools(&tools)) },
            "tool_choice": if tools.is_empty() { Value::Null } else { Value::String("auto".to_string()) }
        });

        let url = format!("{}/chat/completions", self.config.base_url.trim_end_matches('/'));
        let mut last_err: Option<anyhow::Error> = None;
        for attempt in 0..=self.max_retries {
            match self
                .client
                .post(&url)
                .headers(headers.clone())
                .json(&payload)
                .send()
                .await
            {
                Ok(response) => {
                    if !response.status().is_success() {
                        let status = response.status();
                        let body = response.text().await.unwrap_or_default();
                        last_err = Some(anyhow!("API error {}: {}", status, body));
                    } else {
                        let parsed = response.json::<Value>().await?;
                        return Ok(self.parse_non_stream_response(parsed));
                    }
                }
                Err(err) => {
                    last_err = Some(anyhow!(err));
                }
            }
            if attempt < self.max_retries {
                sleep(Duration::from_secs(2_u64.pow(attempt as u32))).await;
            }
        }
        Err(last_err.unwrap_or_else(|| anyhow!("chat completion failed")))
    }

    fn parse_non_stream_response(&self, response: Value) -> Vec<StreamEvent> {
        let mut events = Vec::new();
        let choice = response
            .get("choices")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .cloned()
            .unwrap_or_default();
        let message = choice.get("message").cloned().unwrap_or_default();

        if let Some(content) = message.get("content").and_then(|v| v.as_str()) {
            if !content.is_empty() {
                events.push(StreamEvent {
                    event_type: StreamEventType::TextComplete,
                    data: serde_json::json!({"content": content}),
                });
            }
        }

        if let Some(tool_calls) = message.get("tool_calls").and_then(|v| v.as_array()) {
            for tc in tool_calls {
                let call_id = tc
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                let name = tc
                    .get("function")
                    .and_then(|v| v.get("name"))
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                let raw_args = tc
                    .get("function")
                    .and_then(|v| v.get("arguments"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("{}");
                let args = Value::Object(parse_tool_call_arguments(raw_args));
                events.push(StreamEvent {
                    event_type: StreamEventType::ToolCallStart,
                    data: serde_json::json!({"call_id": call_id, "name": name}),
                });
                events.push(StreamEvent {
                    event_type: StreamEventType::ToolCallComplete,
                    data: serde_json::json!({"call_id": call_id, "name": name, "arguments": args}),
                });
            }
        }

        events.push(StreamEvent {
            event_type: StreamEventType::MessageComplete,
            data: serde_json::json!({
                "finish_reason": choice.get("finish_reason").cloned().unwrap_or(Value::Null),
                "usage": response.get("usage").cloned().unwrap_or(Value::Null),
            }),
        });

        events
    }

    pub async fn close(&self) -> Result<()> {
        Ok(())
    }
}
