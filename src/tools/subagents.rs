use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::time::{Duration, timeout};

use crate::agent::{Agent, AgentEventType};
use crate::config::Config;
use crate::tools::base::{Tool, ToolInvocation, ToolKind, ToolResult};

#[derive(Debug, Deserialize)]
struct SubagentParams {
    goal: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubagentDefinition {
    pub name: String,
    pub description: String,
    pub goal_prompt: String,
    pub allowed_tools: Option<Vec<String>>,
    pub max_turns: u32,
    pub timeout_seconds: f64,
}

pub struct SubagentTool {
    #[allow(dead_code)]
    config: Config,
    definition: SubagentDefinition,
    name: String,
    description: String,
}

impl SubagentTool {
    pub fn new(config: Config, definition: SubagentDefinition) -> Self {
        let name = format!("subagent_{}", definition.name);
        let description = format!("subagent_{}", definition.description);
        Self {
            config,
            definition,
            name,
            description,
        }
    }
}

#[async_trait]
impl Tool for SubagentTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn kind(&self) -> ToolKind {
        ToolKind::Write
    }

    fn schema(&self) -> Value {
        json!({"type":"object","properties":{"goal":{"type":"string"}},"required":["goal"]})
    }

    fn is_mutating(&self, _params: &Value) -> bool {
        true
    }

    async fn execute(&self, invocation: ToolInvocation) -> Result<ToolResult> {
        let params: SubagentParams = serde_json::from_value(invocation.params)?;
        if params.goal.trim().is_empty() {
            return Ok(ToolResult::error_result(
                "No goal specified for sub-agent".to_string(),
            ));
        }

        let mut sub_config = self.config.clone();
        sub_config.max_turns = self.definition.max_turns;
        if let Some(allowed) = &self.definition.allowed_tools {
            sub_config.allowed_tools = Some(allowed.clone());
        }

        let prompt = format!(
            "You are a specialized sub-agent with a specific task to complete.\n\n{}\n\nYOUR TASK:\n{}\n\nIMPORTANT:\n- Focus only on completing the specified task\n- Do not engage in unrelated actions\n- Once complete, provide final response\n- Be concise and direct",
            self.definition.goal_prompt, params.goal
        );

        let mut tool_calls = Vec::new();
        let mut final_response = String::new();
        let mut terminate_reason = "goal".to_string();

        let mut subagent = Agent::new(sub_config).await?;
        let run_future = subagent.run(prompt);
        let result = timeout(
            Duration::from_secs_f64(self.definition.timeout_seconds),
            run_future,
        )
        .await;

        match result {
            Ok(Ok(events)) => {
                for event in events {
                    match event.event_type {
                        AgentEventType::ToolCallStart => {
                            if let Some(name) = event.data.get("name").and_then(|v| v.as_str()) {
                                tool_calls.push(name.to_string());
                            }
                        }
                        AgentEventType::TextComplete => {
                            if let Some(content) = event.data.get("content").and_then(|v| v.as_str()) {
                                final_response = content.to_string();
                            }
                        }
                        AgentEventType::AgentEnd => {
                            if final_response.is_empty() {
                                final_response = event
                                    .data
                                    .get("response")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or_default()
                                    .to_string();
                            }
                        }
                        AgentEventType::AgentError => {
                            terminate_reason = "error".to_string();
                            let err = event
                                .data
                                .get("error")
                                .and_then(|v| v.as_str())
                                .unwrap_or("Unknown");
                            final_response = format!("Sub-agent error: {}", err);
                        }
                        _ => {}
                    }
                }
            }
            Ok(Err(err)) => {
                terminate_reason = "error".to_string();
                final_response = format!("Sub-agent failed: {}", err);
            }
            Err(_) => {
                terminate_reason = "timeout".to_string();
                final_response = "Sub-agent timed out".to_string();
            }
        }

        let result = format!(
            "Sub-agent '{}' completed.\nTermination: {}\nTools called: {}\n\nResult:\n{}",
            self.definition.name,
            terminate_reason,
            if tool_calls.is_empty() {
                "None".to_string()
            } else {
                tool_calls.join(", ")
            },
            if final_response.is_empty() {
                "No response".to_string()
            } else {
                final_response
            }
        );

        if terminate_reason == "error" {
            return Ok(ToolResult::error_result(result));
        }
        Ok(ToolResult::success_result(result))
    }
}

pub fn get_default_subagent_definitions() -> Vec<SubagentDefinition> {
    vec![
        SubagentDefinition {
            name: "codebase_investigator".to_string(),
            description:
                "Investigates the codebase to answer questions about code structure, patterns, and implementations"
                    .to_string(),
            goal_prompt: "Use read_file, grep, glob, and list_dir to investigate. Do not modify any files."
                .to_string(),
            allowed_tools: Some(vec![
                "read_file".to_string(),
                "grep".to_string(),
                "glob".to_string(),
                "list_dir".to_string(),
            ]),
            max_turns: 20,
            timeout_seconds: 600.0,
        },
        SubagentDefinition {
            name: "code_reviewer".to_string(),
            description: "Reviews code changes and provides feedback on quality, bugs, and improvements"
                .to_string(),
            goal_prompt: "Use read_file, list_dir and grep to examine code. Do not modify files."
                .to_string(),
            allowed_tools: Some(vec![
                "read_file".to_string(),
                "grep".to_string(),
                "list_dir".to_string(),
            ]),
            max_turns: 10,
            timeout_seconds: 300.0,
        },
    ]
}

pub fn get_subagent_tools(config: &Config) -> Vec<Arc<dyn Tool>> {
    get_default_subagent_definitions()
        .into_iter()
        .map(|d| Arc::new(SubagentTool::new(config.clone(), d)) as Arc<dyn Tool>)
        .collect()
}
