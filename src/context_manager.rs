use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageItem {
    pub role: String,
    pub content: String,
    pub tool_calls: Option<Value>,
    pub tool_call_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

pub struct ContextManager {
    messages: Vec<MessageItem>,
    pub total_usage: u64,
}

impl ContextManager {
    pub fn new(system_prompt: Option<String>) -> Self {
        let mut s = Self {
            messages: Vec::new(),
            total_usage: 0,
        };
        if let Some(prompt) = system_prompt {
            s.messages.push(MessageItem {
                role: "system".to_string(),
                content: prompt,
                tool_calls: None,
                tool_call_id: None,
                created_at: Utc::now(),
            });
        }
        s
    }

    pub fn add_user_message(&mut self, content: String) {
        self.messages.push(MessageItem {
            role: "user".to_string(),
            content,
            tool_calls: None,
            tool_call_id: None,
            created_at: Utc::now(),
        });
    }

    pub fn add_assistant_message(&mut self, content: String, tool_calls: Option<Value>) {
        self.messages.push(MessageItem {
            role: "assistant".to_string(),
            content,
            tool_calls,
            tool_call_id: None,
            created_at: Utc::now(),
        });
    }

    pub fn add_tool_result(&mut self, tool_call_id: String, content: String) {
        self.messages.push(MessageItem {
            role: "tool".to_string(),
            content,
            tool_calls: None,
            tool_call_id: Some(tool_call_id),
            created_at: Utc::now(),
        });
    }

    pub fn get_messages(&self) -> Vec<Value> {
        self.messages
            .iter()
            .map(|m| serde_json::json!({"role": m.role, "content": m.content, "tool_calls": m.tool_calls, "tool_call_id": m.tool_call_id}))
            .collect()
    }

    pub fn clear(&mut self) {
        self.messages.clear();
        self.total_usage = 0;
    }

    pub fn message_count(&self) -> usize {
        self.messages.len()
    }
}
