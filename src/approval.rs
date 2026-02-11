use std::path::PathBuf;

use crate::config::ApprovalPolicy;

#[derive(Debug, Clone)]
pub enum ApprovalDecision {
    Approved,
    Rejected,
    NeedsConfirmation,
}

#[derive(Debug, Clone)]
pub struct ApprovalContext {
    pub tool_name: String,
    pub is_mutating: bool,
    pub affected_paths: Vec<PathBuf>,
    pub command: Option<String>,
    pub is_dangerous: bool,
}

pub fn is_dangerous_command(command: &str) -> bool {
    let cmd = command.to_lowercase();
    let blocked = [
        "rm -rf /",
        "dd if=/dev/zero",
        "mkfs",
        "shutdown",
        "reboot",
        "halt",
        "poweroff",
    ];
    blocked.iter().any(|x| cmd.contains(x))
}

pub fn is_safe_command(command: &str) -> bool {
    !is_dangerous_command(command)
}

#[derive(Debug, Clone)]
pub struct ApprovalManager {
    policy: ApprovalPolicy,
    #[allow(dead_code)]
    cwd: PathBuf,
}

impl ApprovalManager {
    pub fn new(policy: ApprovalPolicy, cwd: PathBuf) -> Self {
        Self { policy, cwd }
    }

    pub fn check_approval(&self, ctx: &ApprovalContext) -> ApprovalDecision {
        match self.policy {
            ApprovalPolicy::Yolo => ApprovalDecision::Approved,
            ApprovalPolicy::Never => {
                if ctx.is_mutating || ctx.is_dangerous {
                    ApprovalDecision::Rejected
                } else {
                    ApprovalDecision::Approved
                }
            }
            ApprovalPolicy::Auto => {
                if ctx.is_dangerous {
                    ApprovalDecision::NeedsConfirmation
                } else {
                    ApprovalDecision::Approved
                }
            }
            ApprovalPolicy::OnRequest => {
                if ctx.is_mutating || ctx.is_dangerous {
                    ApprovalDecision::NeedsConfirmation
                } else {
                    ApprovalDecision::Approved
                }
            }
        }
    }

    pub fn request_confirmation(&self, _ctx: &ApprovalContext) -> bool {
        false
    }
}
