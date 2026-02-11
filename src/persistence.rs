use std::path::PathBuf;

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSnapshot {
    pub session_id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub turn_count: u32,
    pub messages: Vec<Value>,
    pub total_usage: u64,
}

pub struct PersistenceManager {
    base_dir: PathBuf,
}

impl PersistenceManager {
    pub fn new() -> Self {
        let base = crate::config_loader::get_data_dir().join("sessions");
        Self { base_dir: base }
    }

    fn sessions_dir(&self) -> PathBuf {
        self.base_dir.join("saved")
    }

    fn checkpoints_dir(&self) -> PathBuf {
        self.base_dir.join("checkpoints")
    }

    pub fn save_session(&self, snapshot: &SessionSnapshot) -> Result<()> {
        std::fs::create_dir_all(self.sessions_dir())?;
        let path = self.sessions_dir().join(format!("{}.json", snapshot.session_id));
        std::fs::write(path, serde_json::to_string_pretty(snapshot)?)?;
        Ok(())
    }

    pub fn load_session(&self, session_id: &str) -> Result<Option<SessionSnapshot>> {
        let path = self.sessions_dir().join(format!("{}.json", session_id));
        if !path.exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(path)?;
        Ok(Some(serde_json::from_str(&content)?))
    }

    pub fn list_sessions(&self) -> Result<Vec<Value>> {
        let mut out = Vec::new();
        let dir = self.sessions_dir();
        if !dir.exists() {
            return Ok(out);
        }
        for entry in std::fs::read_dir(dir)? {
            let path = entry?.path();
            if path.extension().and_then(|x| x.to_str()) != Some("json") {
                continue;
            }
            let content = std::fs::read_to_string(&path)?;
            let snapshot: SessionSnapshot = serde_json::from_str(&content)?;
            out.push(serde_json::json!({
                "session_id": snapshot.session_id,
                "turn_count": snapshot.turn_count,
                "updated_at": snapshot.updated_at.to_rfc3339(),
            }));
        }
        Ok(out)
    }

    pub fn save_checkpoint(&self, snapshot: &SessionSnapshot) -> Result<String> {
        std::fs::create_dir_all(self.checkpoints_dir())?;
        let checkpoint_id = format!("{}_{}", snapshot.session_id, snapshot.turn_count);
        let path = self.checkpoints_dir().join(format!("{}.json", checkpoint_id));
        std::fs::write(path, serde_json::to_string_pretty(snapshot)?)?;
        Ok(checkpoint_id)
    }

    pub fn load_checkpoint(&self, checkpoint_id: &str) -> Result<Option<SessionSnapshot>> {
        let path = self.checkpoints_dir().join(format!("{}.json", checkpoint_id));
        if !path.exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(path)?;
        Ok(Some(serde_json::from_str(&content)?))
    }

    pub fn list_checkpoints(&self) -> Result<Vec<Value>> {
        let mut out = Vec::new();
        let dir = self.checkpoints_dir();
        if !dir.exists() {
            return Ok(out);
        }
        for entry in std::fs::read_dir(dir)? {
            let path = entry?.path();
            if path.extension().and_then(|x| x.to_str()) != Some("json") {
                continue;
            }
            out.push(serde_json::json!({
                "checkpoint_id": path.file_stem().and_then(|x| x.to_str()).unwrap_or_default()
            }));
        }
        Ok(out)
    }
}
