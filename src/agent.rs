use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::client_llm::LLMClient;
use crate::config::Config;
use crate::session::Session;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentEventType {
    TextDelta,
    TextComplete,
    ToolCallStart,
    ToolCallComplete,
    AgentError,
    AgentEnd,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentEvent {
    pub event_type: AgentEventType,
    pub data: Value,
}

pub struct Agent {
    pub config: Config,
    pub session: Session,
    #[allow(dead_code)]
    client: LLMClient,
}

impl Agent {
    pub async fn new(config: Config) -> Result<Self> {
        let client = LLMClient::new(config.clone());
        let mut session = Session::new(config.clone());
        session.initialize().await?;
        Ok(Self {
            config,
            session,
            client,
        })
    }

    pub async fn run(&mut self, message: String) -> Result<Vec<AgentEvent>> {
        self.session.context_manager.add_user_message(message.clone());
        self.session.increment_turn();
        let response = format!("placeholder response for: {}", message);
        self.session
            .context_manager
            .add_assistant_message(response.clone(), None);
        Ok(vec![
            AgentEvent {
                event_type: AgentEventType::TextComplete,
                data: serde_json::json!({"content": response}),
            },
            AgentEvent {
                event_type: AgentEventType::AgentEnd,
                data: serde_json::json!({"response":"done"}),
            },
        ])
    }
}
