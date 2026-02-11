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
    ToolCallStart,
    ToolCallComplete,
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
    serde_json::from_str(arguments_str).unwrap_or_default()
}
