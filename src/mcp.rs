use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
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
    pub cwd: PathBuf,
    status: Arc<RwLock<MCPServerStatus>>,
    tools: Arc<RwLock<HashMap<String, MCPToolInfo>>>,
}

impl MCPClient {
    pub fn new(name: String, config: MCPServerConfig, cwd: PathBuf) -> Self {
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

    fn set_status(&self, status: MCPServerStatus) {
        if let Ok(mut s) = self.status.write() {
            *s = status;
        }
    }

    pub async fn connect(&self) -> Result<()> {
        if self.status() == MCPServerStatus::Connected {
            return Ok(());
        }
        self.set_status(MCPServerStatus::Connecting);

        let result = if self.config.command.is_some() {
            self.connect_stdio().await
        } else if self.config.url.is_some() {
            self.connect_http().await
        } else {
            Err(anyhow!("MCP server '{}' has no command or URL", self.name))
        };

        match result {
            Ok(_) => {
                self.set_status(MCPServerStatus::Connected);
                Ok(())
            }
            Err(e) => {
                self.set_status(MCPServerStatus::Error);
                Err(e)
            }
        }
    }

    async fn connect_stdio(&self) -> Result<()> {
        let init = json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"rust-coding-agent","version":"0.1.0"}}});
        let list = json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}});
        let responses = self.run_stdio_requests(vec![init, list]).await?;
        self.load_tools_from_list_responses(&responses);
        Ok(())
    }

    async fn connect_http(&self) -> Result<()> {
        let _ = self
            .send_http_request("initialize", json!({"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"rust-coding-agent","version":"0.1.0"}}))
            .await?;
        let list = self.send_http_request("tools/list", json!({})).await?;
        self.load_tools_from_list_result(list.get("result").cloned().unwrap_or_default());
        Ok(())
    }

    fn load_tools_from_list_responses(&self, responses: &[Value]) {
        for response in responses {
            if response
                .get("id")
                .and_then(|v| v.as_i64())
                .unwrap_or_default()
                == 2
            {
                self.load_tools_from_list_result(response.get("result").cloned().unwrap_or_default());
                return;
            }
        }
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

    fn load_tools_from_list_result(&self, result: Value) {
        let mut map = HashMap::new();
        let tools = result
            .get("tools")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        for t in tools {
            let name = t.get("name").and_then(|v| v.as_str()).unwrap_or_default();
            if name.is_empty() {
                continue;
            }
            let description = t
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let schema = t
                .get("inputSchema")
                .or_else(|| t.get("input_schema"))
                .cloned()
                .unwrap_or_else(|| json!({"type":"object","properties":{}}));
            map.insert(
                name.to_string(),
                MCPToolInfo {
                    name: name.to_string(),
                    description,
                    input_schema: schema,
                    server_name: self.name.clone(),
                },
            );
        }
        if map.is_empty() {
            for t in &self.config.tools {
                map.insert(
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
        if let Ok(mut tools) = self.tools.write() {
            *tools = map;
        }
    }

    async fn run_stdio_requests(&self, requests: Vec<Value>) -> Result<Vec<Value>> {
        let command = self.config.command.clone().unwrap_or_default();
        if command.trim().is_empty() {
            return Err(anyhow!("No stdio command for server {}", self.name));
        }
        #[cfg(target_os = "windows")]
        let mut cmd = {
            let mut c = Command::new("cmd.exe");
            c.arg("/c").arg(&command);
            c
        };
        #[cfg(not(target_os = "windows"))]
        let mut cmd = {
            let mut c = Command::new("/bin/bash");
            c.arg("-c").arg(&command);
            c
        };
        for arg in &self.config.args {
            cmd.arg(arg);
        }
        cmd.current_dir(self.config.cwd.clone().unwrap_or_else(|| self.cwd.clone()));
        cmd.envs(self.config.env.clone());
        cmd.stdin(std::process::Stdio::piped());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        let mut child = cmd.spawn()?;
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow!("failed to open MCP stdio stdin"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow!("failed to open MCP stdio stdout"))?;
        let mut reader = BufReader::new(stdout).lines();

        for req in &requests {
            stdin.write_all(req.to_string().as_bytes()).await?;
            stdin.write_all(b"\n").await?;
        }
        stdin.shutdown().await?;

        let mut responses = Vec::new();
        let expected = requests.len();
        while responses.len() < expected {
            let line = timeout(
                Duration::from_secs(self.config.startup_timeout_sec.max(1)),
                reader.next_line(),
            )
            .await
            .map_err(|_| anyhow!("MCP stdio response timed out"))??;
            let Some(line) = line else {
                break;
            };
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Ok(json) = serde_json::from_str::<Value>(trimmed) {
                responses.push(json);
            }
        }
        Ok(responses)
    }

    async fn send_http_request(&self, method: &str, params: Value) -> Result<Value> {
        let url = self
            .config
            .url
            .clone()
            .ok_or_else(|| anyhow!("No MCP HTTP URL"))?;
        let payload = json!({"jsonrpc":"2.0","id":1,"method":method,"params":params});
        let response = reqwest::Client::builder()
            .timeout(Duration::from_secs(self.config.startup_timeout_sec.max(1)))
            .build()?
            .post(url)
            .json(&payload)
            .send()
            .await?;
        let json = response.json::<Value>().await?;
        Ok(json)
    }

    pub async fn disconnect(&self) -> Result<()> {
        if let Ok(mut tools) = self.tools.write() {
            tools.clear();
        }
        self.set_status(MCPServerStatus::Disconnected);
        Ok(())
    }

    pub async fn call_tool(&self, tool_name: &str, arguments: Value) -> Result<Value> {
        if self.status() != MCPServerStatus::Connected {
            return Err(anyhow!("Not connected to server {}", self.name));
        }

        if self.config.url.is_some() {
            let response = self
                .send_http_request(
                    "tools/call",
                    json!({
                        "name": tool_name,
                        "arguments": arguments
                    }),
                )
                .await?;
            return Ok(self.parse_tools_call_response(response));
        }

        let init = json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"rust-coding-agent","version":"0.1.0"}}});
        let call = json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name": tool_name, "arguments": arguments}});
        let responses = self.run_stdio_requests(vec![init, call]).await?;
        for response in responses {
            if response
                .get("id")
                .and_then(|v| v.as_i64())
                .unwrap_or_default()
                == 2
            {
                return Ok(self.parse_tools_call_response(response));
            }
        }
        Err(anyhow!("MCP stdio tools/call returned no response"))
    }

    fn parse_tools_call_response(&self, response: Value) -> Value {
        let result = response.get("result").cloned().unwrap_or_default();
        let is_error = result
            .get("isError")
            .or_else(|| result.get("is_error"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let mut output = String::new();
        if let Some(content) = result.get("content").and_then(|v| v.as_array()) {
            let mut lines = Vec::new();
            for item in content {
                if let Some(text) = item.get("text").and_then(|v| v.as_str()) {
                    lines.push(text.to_string());
                } else {
                    lines.push(item.to_string());
                }
            }
            output = lines.join("\n");
        }
        if output.is_empty() {
            output = result
                .get("output")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
        }
        json!({
            "output": output,
            "is_error": is_error
        })
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
            out.push(json!({
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
