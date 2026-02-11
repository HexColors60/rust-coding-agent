use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MCPServerStatus {
    Connected,
    Disconnected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPToolInfo {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct MCPClient {
    pub name: String,
    pub status: MCPServerStatus,
    pub tools: Vec<MCPToolInfo>,
}

impl MCPClient {
    pub fn new(name: String) -> Self {
        Self {
            name,
            status: MCPServerStatus::Disconnected,
            tools: vec![],
        }
    }

    pub async fn connect(&mut self) -> Result<()> {
        self.status = MCPServerStatus::Connected;
        Ok(())
    }
}

#[derive(Default)]
pub struct MCPManager {
    clients: Vec<MCPClient>,
}

impl MCPManager {
    pub fn new() -> Self {
        Self { clients: vec![] }
    }

    pub async fn initialize(&mut self) -> Result<()> {
        Ok(())
    }

    pub async fn shutdown(&mut self) -> Result<()> {
        Ok(())
    }

    pub fn get_all_servers(&self) -> Vec<serde_json::Value> {
        self.clients
            .iter()
            .map(|c| {
                serde_json::json!({
                    "name": c.name,
                    "status": match c.status { MCPServerStatus::Connected => "connected", MCPServerStatus::Disconnected => "disconnected" },
                    "tools": c.tools.len(),
                })
            })
            .collect()
    }
}
