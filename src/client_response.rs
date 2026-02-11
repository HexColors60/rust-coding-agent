use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextDelta {
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StreamEventType {
    TextDelta,
    TextComplete,
    ToolCallDelta,
    ToolCallStart,
    ToolCallComplete,
    MessageComplete,
    Error,
    AgentError,
    AgentEnd,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallDelta {
    pub id: String,
    pub name: String,
    pub arguments_delta: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamEvent {
    pub event_type: StreamEventType,
    pub data: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResultMessage {
    pub tool_call_id: String,
    pub content: String,
}

pub fn parse_tool_call_arguments(arguments_str: &str) -> serde_json::Map<String, Value> {
    if arguments_str.trim().is_empty() {
        return serde_json::Map::new();
    }
    match serde_json::from_str::<Value>(arguments_str) {
        Ok(Value::Object(map)) => map,
        Ok(other) => {
            let mut map = serde_json::Map::new();
            map.insert("value".to_string(), other);
            map
        }
        Err(_) => {
            let mut map = serde_json::Map::new();
            map.insert(
                "raw".to_string(),
                Value::String(arguments_str.to_string()),
            );
            map
        }
    }
}
