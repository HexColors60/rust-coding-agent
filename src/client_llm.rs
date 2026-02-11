use anyhow::{Result, anyhow};
use futures_util::StreamExt;
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
        let base_payload = serde_json::json!({
            "model": self.config.model_name,
            "messages": messages,
            "temperature": self.config.temperature,
            "tools": if tools.is_empty() { Value::Null } else { Value::Array(self.build_tools(&tools)) },
            "tool_choice": if tools.is_empty() { Value::Null } else { Value::String("auto".to_string()) }
        });

        let url = format!("{}/chat/completions", self.config.base_url.trim_end_matches('/'));
        let mut last_err: Option<anyhow::Error> = None;
        for attempt in 0..=self.max_retries {
            let mut payload = base_payload.clone();
            payload["stream"] = Value::Bool(true);
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
                        match self.parse_stream_response(response).await {
                            Ok(events) if !events.is_empty() => return Ok(events),
                            Ok(_) => {
                                let mut payload = base_payload.clone();
                                payload["stream"] = Value::Bool(false);
                                let response = self
                                    .client
                                    .post(&url)
                                    .headers(headers.clone())
                                    .json(&payload)
                                    .send()
                                    .await?;
                                let parsed = response.json::<Value>().await?;
                                return Ok(self.parse_non_stream_response(parsed));
                            }
                            Err(_) => {
                                let mut payload = base_payload.clone();
                                payload["stream"] = Value::Bool(false);
                                let response = self
                                    .client
                                    .post(&url)
                                    .headers(headers.clone())
                                    .json(&payload)
                                    .send()
                                    .await?;
                                let parsed = response.json::<Value>().await?;
                                return Ok(self.parse_non_stream_response(parsed));
                            }
                        }
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

    async fn parse_stream_response(&self, response: reqwest::Response) -> Result<Vec<StreamEvent>> {
        let mut events = Vec::new();
        let mut buffer = String::new();
        let mut tool_calls = std::collections::HashMap::<i64, (String, String, String)>::new();

        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            let text = String::from_utf8_lossy(&chunk);
            buffer.push_str(&text);

            while let Some(pos) = buffer.find('\n') {
                let line = buffer[..pos].trim_end_matches('\r').trim().to_string();
                buffer = buffer[pos + 1..].to_string();
                if !line.starts_with("data:") {
                    continue;
                }
                let payload = line.trim_start_matches("data:").trim();
                if Self::apply_stream_payload(payload, &mut events, &mut tool_calls) {
                    buffer.clear();
                    break;
                }
            }
        }

        Self::emit_tool_call_complete_events(&mut events, tool_calls);
        Ok(events)
    }

    fn apply_stream_payload(
        payload: &str,
        events: &mut Vec<StreamEvent>,
        tool_calls: &mut std::collections::HashMap<i64, (String, String, String)>,
    ) -> bool {
        if payload == "[DONE]" {
            return true;
        }
        let Ok(v) = serde_json::from_str::<Value>(payload) else {
            return false;
        };
        let choice = v
            .get("choices")
            .and_then(|x| x.as_array())
            .and_then(|x| x.first())
            .cloned()
            .unwrap_or_default();
        let delta = choice.get("delta").cloned().unwrap_or_default();

        if let Some(content) = delta.get("content").and_then(|x| x.as_str()) {
            events.push(StreamEvent {
                event_type: StreamEventType::TextDelta,
                data: serde_json::json!({"content": content}),
            });
        }

        if let Some(tc_arr) = delta.get("tool_calls").and_then(|x| x.as_array()) {
            for tc in tc_arr {
                let idx = tc.get("index").and_then(|x| x.as_i64()).unwrap_or(0);
                let entry = tool_calls.entry(idx).or_insert_with(|| {
                    (
                        tc.get("id")
                            .and_then(|x| x.as_str())
                            .unwrap_or_default()
                            .to_string(),
                        String::new(),
                        String::new(),
                    )
                });
                if entry.0.is_empty() {
                    entry.0 = tc
                        .get("id")
                        .and_then(|x| x.as_str())
                        .unwrap_or_default()
                        .to_string();
                }
                if let Some(name) = tc
                    .get("function")
                    .and_then(|f| f.get("name"))
                    .and_then(|x| x.as_str())
                {
                    entry.1 = name.to_string();
                    events.push(StreamEvent {
                        event_type: StreamEventType::ToolCallStart,
                        data: serde_json::json!({"call_id": entry.0, "name": entry.1}),
                    });
                }
                if let Some(delta_args) = tc
                    .get("function")
                    .and_then(|f| f.get("arguments"))
                    .and_then(|x| x.as_str())
                {
                    entry.2.push_str(delta_args);
                    events.push(StreamEvent {
                        event_type: StreamEventType::ToolCallDelta,
                        data: serde_json::json!({"call_id": entry.0, "name": entry.1, "arguments_delta": delta_args}),
                    });
                }
            }
        }

        if let Some(reason) = choice.get("finish_reason").and_then(|x| x.as_str()) {
            events.push(StreamEvent {
                event_type: StreamEventType::MessageComplete,
                data: serde_json::json!({"finish_reason": reason}),
            });
        }
        false
    }

    fn emit_tool_call_complete_events(
        events: &mut Vec<StreamEvent>,
        tool_calls: std::collections::HashMap<i64, (String, String, String)>,
    ) {
        let mut entries: Vec<(i64, (String, String, String))> = tool_calls.into_iter().collect();
        entries.sort_by_key(|(idx, _)| *idx);
        for (_, (call_id, name, raw_args)) in entries {
            events.push(StreamEvent {
                event_type: StreamEventType::ToolCallComplete,
                data: serde_json::json!({
                    "call_id": call_id,
                    "name": name,
                    "arguments": Value::Object(parse_tool_call_arguments(&raw_args))
                }),
            });
        }
    }

    pub async fn close(&self) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_stream_payload_collects_deltas_and_tool_chunks() {
        let mut events = Vec::new();
        let mut tool_calls = std::collections::HashMap::new();
        let payload1 = r#"{"choices":[{"delta":{"content":"Hello"},"finish_reason":null}]}"#;
        let payload2 = r#"{"choices":[{"delta":{"tool_calls":[{"index":0,"id":"call_1","function":{"name":"read_file","arguments":"{\"path\":\"a"}}]}}]}"#;
        let payload3 = r#"{"choices":[{"delta":{"tool_calls":[{"index":0,"function":{"arguments":".rs\"}"}}]},"finish_reason":"tool_calls"}]}"#;

        assert!(!LLMClient::apply_stream_payload(
            payload1,
            &mut events,
            &mut tool_calls
        ));
        assert!(!LLMClient::apply_stream_payload(
            payload2,
            &mut events,
            &mut tool_calls
        ));
        assert!(!LLMClient::apply_stream_payload(
            payload3,
            &mut events,
            &mut tool_calls
        ));

        LLMClient::emit_tool_call_complete_events(&mut events, tool_calls);
        assert!(
            events
                .iter()
                .any(|e| matches!(e.event_type, StreamEventType::TextDelta))
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e.event_type, StreamEventType::ToolCallDelta))
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e.event_type, StreamEventType::ToolCallComplete))
        );
    }

    #[test]
    fn apply_stream_payload_done_flag() {
        let mut events = Vec::new();
        let mut tool_calls = std::collections::HashMap::new();
        assert!(LLMClient::apply_stream_payload(
            "[DONE]",
            &mut events,
            &mut tool_calls
        ));
    }
}
