use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use anyhow::Result;
use serde_json::Value;

#[derive(Default, Clone)]
pub struct ToolDiscoveryManager {
    discovered_manifests: Arc<RwLock<Vec<PathBuf>>>,
}

impl ToolDiscoveryManager {
    pub fn new() -> Self {
        Self {
            discovered_manifests: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn discover_all(&self) -> Result<()> {
        let cwd = std::env::current_dir()?;
        let roots = [
            cwd.join("tools").join("discovered"),
            cwd.join(".agent").join("tools"),
        ];
        let mut discovered = Vec::new();

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
                let json: Value = serde_json::from_str(&content)?;
                let valid = json.get("name").is_some()
                    && json.get("description").is_some()
                    && json.get("parameters").is_some();
                if valid {
                    discovered.push(path.to_path_buf());
                }
            }
        }

        if let Ok(mut out) = self.discovered_manifests.write() {
            *out = discovered;
        }
        Ok(())
    }

    pub fn discovered_files(&self) -> Vec<PathBuf> {
        self.discovered_manifests
            .read()
            .map(|v| v.clone())
            .unwrap_or_default()
    }
}
