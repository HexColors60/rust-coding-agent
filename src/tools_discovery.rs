use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use anyhow::Result;

use crate::config::Config;
use crate::tools::base::Tool;
use crate::tools::discovered_tool::{DiscoveredTool, DiscoveredToolSpec};

#[derive(Default, Clone)]
pub struct ToolDiscoveryManager {
    discovered_specs: Arc<RwLock<Vec<DiscoveredToolSpec>>>,
    discovered_manifests: Arc<RwLock<Vec<PathBuf>>>,
}

impl ToolDiscoveryManager {
    pub fn new() -> Self {
        Self {
            discovered_specs: Arc::new(RwLock::new(Vec::new())),
            discovered_manifests: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn discover_all(&self, cwd: &Path) -> Result<()> {
        let roots = [
            cwd.join("tools").join("discovered"),
            cwd.join(".agent").join("tools"),
        ];
        let mut discovered_specs = Vec::new();
        let mut discovered_paths = Vec::new();

        for root in roots {
            if !root.exists() {
                continue;
            }
            for entry in walkdir::WalkDir::new(&root).into_iter().filter_map(|e| e.ok()) {
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }
                if path.extension().and_then(|s| s.to_str()) != Some("json") {
                    continue;
                }
                let content = std::fs::read_to_string(path)?;
                let spec: DiscoveredToolSpec = serde_json::from_str(&content)?;
                if spec.name.trim().is_empty()
                    || spec.description.trim().is_empty()
                    || spec.command.trim().is_empty()
                {
                    continue;
                }
                discovered_specs.push(spec);
                discovered_paths.push(path.to_path_buf());
            }
        }

        if let Ok(mut out) = self.discovered_specs.write() {
            *out = discovered_specs;
        }
        if let Ok(mut out) = self.discovered_manifests.write() {
            *out = discovered_paths;
        }
        Ok(())
    }

    pub fn build_tools(&self, config: &Config) -> Vec<Arc<dyn Tool>> {
        let specs = self
            .discovered_specs
            .read()
            .map(|v| v.clone())
            .unwrap_or_default();
        let mut tools: Vec<Arc<dyn Tool>> = Vec::new();
        for mut spec in specs {
            if spec.cwd.is_none() {
                spec.cwd = Some(config.cwd.clone());
            }
            tools.push(Arc::new(DiscoveredTool::new(spec)));
        }
        tools
    }

    pub fn discovered_files(&self) -> Vec<PathBuf> {
        self.discovered_manifests
            .read()
            .map(|v| v.clone())
            .unwrap_or_default()
    }
}
