use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::config::Config;
use crate::tools::base::{Tool, ToolInvocation, ToolKind, ToolResult};

#[derive(Debug, Deserialize)]
struct TodosParams {
    action: String,
    id: Option<String>,
    content: Option<String>,
}

pub struct TodosTool {
    #[allow(dead_code)]
    config: Config,
    todos: Arc<Mutex<HashMap<String, String>>>,
}

impl TodosTool {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            todos: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl Tool for TodosTool {
    fn name(&self) -> &str {
        "todos"
    }

    fn description(&self) -> &str {
        "Manage a task list for the current session."
    }

    fn kind(&self) -> ToolKind {
        ToolKind::Memory
    }

    fn schema(&self) -> Value {
        json!({"type":"object","properties":{"action":{"type":"string"},"id":{"type":"string"},"content":{"type":"string"}},"required":["action"]})
    }

    async fn execute(&self, invocation: ToolInvocation) -> Result<ToolResult> {
        let params: TodosParams = serde_json::from_value(invocation.params)?;
        let mut todos = self.todos.lock().expect("mutex poisoned");
        match params.action.to_lowercase().as_str() {
            "add" => {
                let Some(content) = params.content else {
                    return Ok(ToolResult::error_result("`content` required for 'add' action".to_string()));
                };
                let id = Uuid::new_v4().to_string()[..8].to_string();
                todos.insert(id.clone(), content.clone());
                Ok(ToolResult::success_result(format!("Added todo [{}]: {}", id, content)))
            }
            "complete" => {
                let Some(id) = params.id else {
                    return Ok(ToolResult::error_result("`id` required for 'complete' action".to_string()));
                };
                let Some(content) = todos.remove(&id) else {
                    return Ok(ToolResult::error_result(format!("Todo not found: {}", id)));
                };
                Ok(ToolResult::success_result(format!("Completed todo [{}]: {}", id, content)))
            }
            "list" => {
                if todos.is_empty() {
                    return Ok(ToolResult::success_result("No todos".to_string()));
                }
                let mut lines = vec!["Todos:".to_string()];
                for (id, content) in todos.iter() {
                    lines.push(format!("  [{}] {}", id, content));
                }
                Ok(ToolResult::success_result(lines.join("\n")))
            }
            "clear" => {
                let count = todos.len();
                todos.clear();
                Ok(ToolResult::success_result(format!("Cleared {} todos", count)))
            }
            _ => Ok(ToolResult::error_result(format!("Unknown action: {}", params.action))),
        }
    }
}
