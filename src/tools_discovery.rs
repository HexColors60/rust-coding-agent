use anyhow::Result;

#[derive(Default)]
pub struct ToolDiscoveryManager;

impl ToolDiscoveryManager {
    pub fn new() -> Self {
        Self
    }

    pub fn discover_all(&self) -> Result<()> {
        Ok(())
    }
}
