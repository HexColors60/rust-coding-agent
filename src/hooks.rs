use anyhow::Result;
use serde_json::Value;

#[derive(Debug, Clone)]
pub enum HookTrigger {
    BeforeRun,
    AfterRun,
    BeforeTool,
    AfterTool,
    OnError,
}

#[derive(Default)]
pub struct HookSystem;

impl HookSystem {
    pub fn new() -> Self {
        Self
    }

    pub async fn trigger_before_run(&self) -> Result<()> {
        Ok(())
    }

    pub async fn trigger_after_run(&self) -> Result<()> {
        Ok(())
    }

    pub async fn trigger_before_tool(&self, _tool_name: &str, _params: &Value) -> Result<()> {
        Ok(())
    }

    pub async fn trigger_after_tool(
        &self,
        _tool_name: &str,
        _params: &Value,
        _result: &Value,
    ) -> Result<()> {
        Ok(())
    }

    pub async fn trigger_on_error(&self, _error: &str) -> Result<()> {
        Ok(())
    }
}
