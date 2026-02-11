use std::collections::HashMap;
use std::process::Stdio;
use std::sync::{Arc, RwLock};

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::process::Command;
use tokio::time::{Duration, timeout};

use crate::config::{Config, MCPServerConfig};
use crate::registry::ToolRegistry;
use crate::tools::mcp_tool::MCPTool;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MCPServerStatus {
    Disconnected,
    Connecting,
    Connected,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPToolInfo {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
    pub server_name: String,
}

#[derive(Debug, Clone)]
pub struct MCPClient {
    pub name: String,
    pub config: MCPServerConfig,
    pub cwd: std::path::PathBuf,
    status: Arc<RwLock<MCPServerStatus>>,
    tools: Arc<RwLock<HashMap<String, MCPToolInfo>>>,
}

impl MCPClient {
    pub fn new(name: String, config: MCPServerConfig, cwd: std::path::PathBuf) -> Self {
        Self {
            name,
            config,
            cwd,
            status: Arc::new(RwLock::new(MCPServerStatus::Disconnected)),
            tools: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn status(&self) -> MCPServerStatus {
        self.status
            .read()
            .map(|v| v.clone())
            .unwrap_or(MCPServerStatus::Error)
    }

    pub fn tools(&self) -> Vec<MCPToolInfo> {
        self.tools
            .read()
            .map(|v| v.values().cloned().collect())
            .unwrap_or_default()
    }

    pub async fn connect(&self) -> Result<()> {
        {
            let mut s = self.status.write().map_err(|_| anyhow!("lock poisoned"))?;
            if *s == MCPServerStatus::Connected {
                return Ok(());
            }
            *s = MCPServerStatus::Connecting;
        }

        let connect_result = if self.config.command.is_some() {
            self.connect_stdio().await
        } else if self.config.url.is_some() {
            self.connect_http().await
        } else {
            Err(anyhow!("MCP server '{}' has no command or URL", self.name))
        };

        match connect_result {
            Ok(_) => {
                let mut s = self.status.write().map_err(|_| anyhow!("lock poisoned"))?;
                *s = MCPServerStatus::Connected;
                Ok(())
            }
            Err(e) => {
                let mut s = self.status.write().map_err(|_| anyhow!("lock poisoned"))?;
                *s = MCPServerStatus::Error;
                Err(e)
            }
        }
    }

    async fn connect_stdio(&self) -> Result<()> {
        let command = self.config.command.clone().unwrap_or_default();
        #[cfg(target_os = "windows")]
        let mut cmd = {
            let mut c = Command::new("cmd.exe");
            c.arg("/c").arg(command);
            c
        };
        #[cfg(not(target_os = "windows"))]
        let mut cmd = {
            let mut c = Command::new("/bin/bash");
            c.arg("-c").arg(command);
            c
        };
        cmd.current_dir(self.config.cwd.clone().unwrap_or_else(|| self.cwd.clone()));
        cmd.envs(self.config.env.clone());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        let mut child = cmd.spawn()?;
        let _ = timeout(
            Duration::from_secs(self.config.startup_timeout_sec.max(1)),
            child.wait(),
        )
        .await;
        self.load_tools_from_config();
        Ok(())
    }

    async fn connect_http(&self) -> Result<()> {
        let url = self.config.url.clone().unwrap_or_default();
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(self.config.startup_timeout_sec.max(1)))
            .build()?;
        let _ = client.get(url).send().await?;
        self.load_tools_from_config();
        Ok(())
    }

    fn load_tools_from_config(&self) {
        if let Ok(mut tools) = self.tools.write() {
            tools.clear();
            for t in &self.config.tools {
                tools.insert(
                    t.name.clone(),
                    MCPToolInfo {
                        name: t.name.clone(),
                        description: t.description.clone(),
                        input_schema: t.input_schema.clone(),
                        server_name: self.name.clone(),
                    },
                );
            }
        }
    }

    pub async fn disconnect(&self) -> Result<()> {
        if let Ok(mut tools) = self.tools.write() {
            tools.clear();
        }
        if let Ok(mut status) = self.status.write() {
            *status = MCPServerStatus::Disconnected;
        }
        Ok(())
    }

    pub async fn call_tool(&self, tool_name: &str, arguments: Value) -> Result<Value> {
        if self.status() != MCPServerStatus::Connected {
            return Err(anyhow!("Not connected to server {}", self.name));
        }

        if let Some(url) = &self.config.url {
            let client = reqwest::Client::builder()
                .timeout(Duration::from_secs(self.config.startup_timeout_sec.max(1)))
                .build()?;
            let payload = serde_json::json!({
                "tool": tool_name,
                "arguments": arguments
            });
            let response = client.post(url).json(&payload).send().await?;
            let json = response.json::<Value>().await.unwrap_or_else(|_| {
                serde_json::json!({"output":"", "is_error": true, "error":"invalid json response"})
            });
            return Ok(json);
        }

        Ok(serde_json::json!({
            "output": format!("MCP stdio tool '{}' invoked (placeholder)", tool_name),
            "is_error": false
        }))
    }
}

#[derive(Default)]
pub struct MCPManager {
    clients: HashMap<String, MCPClient>,
    initialized: bool,
}

impl MCPManager {
    pub fn new() -> Self {
        Self {
            clients: HashMap::new(),
            initialized: false,
        }
    }

    pub async fn initialize(&mut self, config: &Config) -> Result<()> {
        if self.initialized {
            return Ok(());
        }
        for server in &config.mcp_servers {
            if !server.enabled {
                continue;
            }
            let client = MCPClient::new(server.name.clone(), server.clone(), config.cwd.clone());
            let connect = timeout(
                Duration::from_secs(server.startup_timeout_sec.max(1)),
                client.connect(),
            )
            .await;
            if connect.is_ok() {
                self.clients.insert(server.name.clone(), client);
            }
        }
        self.initialized = true;
        Ok(())
    }

    pub async fn register_tools(&self, registry: &mut ToolRegistry) -> usize {
        let mut count = 0;
        for client in self.clients.values() {
            if client.status() != MCPServerStatus::Connected {
                continue;
            }
            for tool_info in client.tools() {
                let name = format!("{}__{}", client.name, tool_info.name);
                let tool = MCPTool::new(client.clone(), tool_info, name);
                registry.register_mcp_tool(Arc::new(tool));
                count += 1;
            }
        }
        count
    }

    pub async fn shutdown(&mut self) -> Result<()> {
        for client in self.clients.values() {
            let _ = client.disconnect().await;
        }
        self.clients.clear();
        self.initialized = false;
        Ok(())
    }

    pub fn get_all_servers(&self) -> Vec<Value> {
        let mut out = Vec::new();
        for (name, client) in &self.clients {
            out.push(serde_json::json!({
                "name": name,
                "status": match client.status() {
                    MCPServerStatus::Connected => "connected",
                    MCPServerStatus::Connecting => "connecting",
                    MCPServerStatus::Disconnected => "disconnected",
                    MCPServerStatus::Error => "error",
                },
                "tools": client.tools().len(),
            }));
        }
        out
    }
}
