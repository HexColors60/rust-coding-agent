use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::client_llm::LLMClient;
use crate::client_response::StreamEventType;
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
        let mut events = Vec::new();
        self.session.hook_system.trigger_before_agent(&message).await?;
        self.session.context_manager.add_user_message(message.clone());
        self.session.increment_turn();

        let mut last_assistant_text = String::new();
        for _ in 0..self.config.max_turns {
            let messages = self.session.context_manager.get_messages();
            let tools = self.session.tool_registry.get_schemas();
            let response_events = match self.client.chat(messages, tools).await {
                Ok(v) => v,
                Err(err) => {
                    let err_msg = err.to_string();
                    let _ = self.session.hook_system.trigger_on_error(&err_msg).await;
                    events.push(AgentEvent {
                        event_type: AgentEventType::AgentError,
                        data: serde_json::json!({"error": err_msg}),
                    });
                    return Ok(events);
                }
            };

            let mut tool_calls: Vec<(String, String, Value)> = Vec::new();
            let mut assistant_text = String::new();

            for event in response_events {
                match event.event_type {
                    StreamEventType::TextDelta => {
                        if let Some(content) = event.data.get("content").and_then(|v| v.as_str()) {
                            assistant_text.push_str(content);
                            events.push(AgentEvent {
                                event_type: AgentEventType::TextDelta,
                                data: serde_json::json!({"content": content}),
                            });
                        }
                    }
                    StreamEventType::TextComplete => {
                        if let Some(content) = event.data.get("content").and_then(|v| v.as_str()) {
                            assistant_text = content.to_string();
                            events.push(AgentEvent {
                                event_type: AgentEventType::TextComplete,
                                data: serde_json::json!({"content": content}),
                            });
                        }
                    }
                    StreamEventType::ToolCallComplete => {
                        let call_id = event
                            .data
                            .get("call_id")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_string();
                        let name = event
                            .data
                            .get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_string();
                        let args = event.data.get("arguments").cloned().unwrap_or(Value::Null);
                        tool_calls.push((call_id, name, args));
                    }
                    StreamEventType::Error => {
                        let err_msg = event
                            .data
                            .get("error")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Unknown model error")
                            .to_string();
                        let _ = self.session.hook_system.trigger_on_error(&err_msg).await;
                        events.push(AgentEvent {
                            event_type: AgentEventType::AgentError,
                            data: serde_json::json!({"error": err_msg}),
                        });
                        return Ok(events);
                    }
                    _ => {}
                }
            }

            if !assistant_text.is_empty() {
                last_assistant_text = assistant_text.clone();
                self.session
                    .context_manager
                    .add_assistant_message(assistant_text, None);
            }

            if tool_calls.is_empty() {
                events.push(AgentEvent {
                    event_type: AgentEventType::AgentEnd,
                    data: serde_json::json!({"response": last_assistant_text}),
                });
                let _ = self
                    .session
                    .hook_system
                    .trigger_after_agent(&message, &last_assistant_text)
                    .await;
                return Ok(events);
            }

            for (call_id, name, args) in tool_calls {
                events.push(AgentEvent {
                    event_type: AgentEventType::ToolCallStart,
                    data: serde_json::json!({"call_id": call_id, "name": name, "arguments": args}),
                });

                let _ = self
                    .session
                    .hook_system
                    .trigger_before_tool(&name, &args)
                    .await;
                let result = self
                    .session
                    .tool_registry
                    .invoke(
                        &name,
                        args.clone(),
                        &self.session.config.cwd,
                        Some(&self.session.approval_manager),
                    )
                    .await?;
                let _ = self
                    .session
                    .hook_system
                    .trigger_after_tool(&name, &args, &result)
                    .await;

                self.session
                    .context_manager
                    .add_tool_result(call_id.clone(), result.to_model_output());
                events.push(AgentEvent {
                    event_type: AgentEventType::ToolCallComplete,
                    data: serde_json::json!({
                        "call_id": call_id,
                        "name": name,
                        "success": result.success,
                        "output": result.output,
                        "error": result.error,
                        "metadata": result.metadata,
                        "diff": result.diff,
                        "truncated": result.truncated,
                        "exit_code": result.exit_code
                    }),
                });
            }
        }

        events.push(AgentEvent {
            event_type: AgentEventType::AgentError,
            data: serde_json::json!({"error":"Max turns reached"}),
        });
        Ok(events)
    }
}
