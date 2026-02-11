use std::path::PathBuf;

use anyhow::Result;
use clap::ValueEnum;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum ApprovalPolicy {
    #[value(name = "on-request")]
    OnRequest,
    #[value(name = "auto")]
    Auto,
    #[value(name = "never")]
    Never,
    #[value(name = "yolo")]
    Yolo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub model_name: String,
    pub temperature: f32,
    pub approval: ApprovalPolicy,
    pub cwd: PathBuf,
    pub max_turns: u32,
    pub hooks_enabled: bool,
    pub allowed_tools: Option<Vec<String>>,
}

impl Config {
    pub fn new(cwd: Option<PathBuf>) -> Result<Self> {
        let cwd = match cwd {
            Some(v) => v,
            None => std::env::current_dir()?,
        };

        Ok(Self {
            model_name: "gpt-5".to_string(),
            temperature: 0.2,
            approval: ApprovalPolicy::OnRequest,
            cwd,
            max_turns: 40,
            hooks_enabled: true,
            allowed_tools: None,
        })
    }

    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();
        if !(0.0..=2.0).contains(&self.temperature) {
            errors.push("temperature must be in [0.0, 2.0]".to_string());
        }
        if self.max_turns == 0 {
            errors.push("max_turns must be > 0".to_string());
        }
        if !self.cwd.exists() {
            errors.push(format!("cwd does not exist: {}", self.cwd.display()));
        }
        errors
    }
}
