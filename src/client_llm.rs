use anyhow::Result;
use serde_json::Value;

use crate::client_response::{StreamEvent, StreamEventType};
use crate::config::Config;

pub struct LLMClient {
    #[allow(dead_code)]
    config: Config,
}

impl LLMClient {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    pub async fn chat(&self, _messages: Vec<Value>, _tools: Vec<Value>) -> Result<Vec<StreamEvent>> {
        Ok(vec![StreamEvent {
            event_type: StreamEventType::TextComplete,
            data: serde_json::json!({"content":"LLM client placeholder response"}),
        }])
    }

    pub async fn close(&self) -> Result<()> {
        Ok(())
    }
}
