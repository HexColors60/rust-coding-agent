use std::io::{self, Write};
use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

use crate::agent::{Agent, AgentEventType};
use crate::config::{ApprovalPolicy, Config};
use crate::persistence::{PersistenceManager, SessionSnapshot};
use crate::session::Session;
use crate::tui::Tui;

#[derive(Parser, Debug)]
#[command(name = "rust-coding-agent")]
pub struct Args {
    #[arg()]
    pub prompt: Option<String>,
    #[arg(long, short = 'c')]
    pub cwd: Option<PathBuf>,
}

pub struct Cli {
    pub agent: Option<Agent>,
    pub config: Config,
    pub tui: Tui,
}

impl Cli {
    pub fn new(config: Config) -> Self {
        Self {
            agent: None,
            config,
            tui: Tui::new(),
        }
    }

    pub async fn run_single(&mut self, message: String) -> Result<Option<String>> {
        let mut agent = Agent::new(self.config.clone()).await?;
        let resp = self.process_message(&mut agent, message).await?;
        self.agent = Some(agent);
        Ok(resp)
    }

    pub async fn run_interactive(&mut self) -> Result<Option<String>> {
        self.tui.print_welcome(
            "AI Agent",
            &[
                format!("model: {}", self.config.model_name),
                format!("cwd: {}", self.config.cwd.display()),
                "commands: /help /config /approval /model /exit".to_string(),
            ],
        );
        let mut agent = Agent::new(self.config.clone()).await?;
        let mut last = None;
        loop {
            print!("\n[user]> ");
            io::stdout().flush()?;
            let mut input = String::new();
            let n = io::stdin().read_line(&mut input)?;
            if n == 0 {
                break;
            }
            let user_input = input.trim().to_string();
            if user_input.is_empty() {
                continue;
            }
            if user_input.starts_with('/') {
                let should_continue = self.handle_command(&mut agent, &user_input).await?;
                if !should_continue {
                    break;
                }
                continue;
            }
            last = self.process_message(&mut agent, user_input).await?;
        }
        self.agent = Some(agent);
        Ok(last)
    }

    async fn process_message(&self, agent: &mut Agent, message: String) -> Result<Option<String>> {
        let mut final_response = None;
        for event in agent.run(message).await? {
            match event.event_type {
                AgentEventType::TextDelta => {
                    self.tui.stream_assistant_delta(event.data.get("content").and_then(|v| v.as_str()).unwrap_or(""));
                }
                AgentEventType::TextComplete => {
                    let content = event
                        .data
                        .get("content")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    println!("{}", content);
                    final_response = Some(content);
                }
                AgentEventType::ToolCallStart => {}
                AgentEventType::ToolCallComplete => {}
                AgentEventType::AgentError => {
                    eprintln!(
                        "Error: {}",
                        event.data.get("error").and_then(|v| v.as_str()).unwrap_or("unknown")
                    );
                }
                AgentEventType::AgentEnd => {}
            }
        }
        Ok(final_response)
    }

    async fn handle_command(&mut self, agent: &mut Agent, command: &str) -> Result<bool> {
        let cmd = command.trim();
        let mut parts = cmd.splitn(2, ' ');
        let cmd_name = parts.next().unwrap_or("");
        let cmd_args = parts.next().unwrap_or("").trim();
        match cmd_name {
            "/exit" | "/quit" => return Ok(false),
            "/help" => self.tui.show_help(),
            "/clear" => {
                agent.session.context_manager.clear();
                agent.session.loop_detector.clear();
                println!("Conversation cleared");
            }
            "/config" => {
                println!("Model: {}", self.config.model_name);
                println!("Temperature: {}", self.config.temperature);
                println!("Approval: {:?}", self.config.approval);
                println!("Working Dir: {}", self.config.cwd.display());
                println!("Max Turns: {}", self.config.max_turns);
                println!("Hooks Enabled: {}", self.config.hooks_enabled);
            }
            "/model" => {
                if !cmd_args.is_empty() {
                    self.config.model_name = cmd_args.to_string();
                    println!("Model changed to: {}", cmd_args);
                } else {
                    println!("Current model: {}", self.config.model_name);
                }
            }
            "/approval" => {
                if !cmd_args.is_empty() {
                    self.config.approval = match cmd_args {
                        "on-request" => ApprovalPolicy::OnRequest,
                        "auto" => ApprovalPolicy::Auto,
                        "never" => ApprovalPolicy::Never,
                        "yolo" => ApprovalPolicy::Yolo,
                        _ => {
                            println!("Incorrect approval policy: {}", cmd_args);
                            self.config.approval.clone()
                        }
                    };
                    println!("Approval policy changed");
                } else {
                    println!("Current approval policy: {:?}", self.config.approval);
                }
            }
            "/stats" => {
                println!("{}", agent.session.get_stats());
            }
            "/tools" => {
                let tools = agent.session.tool_registry.get_tools();
                println!("Available tools ({})", tools.len());
                for tool in tools {
                    println!("{}", tool.name());
                }
            }
            "/mcp" => {
                let mcp_servers = agent.session.mcp_manager.get_all_servers();
                println!("MCP Servers ({})", mcp_servers.len());
                for s in mcp_servers {
                    println!("{}", s);
                }
            }
            "/save" => {
                let pm = PersistenceManager::new();
                let snap = SessionSnapshot {
                    session_id: agent.session.session_id.clone(),
                    created_at: agent.session.created_at,
                    updated_at: agent.session.updated_at,
                    turn_count: agent.session.turn_count,
                    messages: agent.session.context_manager.get_messages(),
                    total_usage: agent.session.context_manager.total_usage,
                };
                pm.save_session(&snap)?;
                println!("Session saved: {}", agent.session.session_id);
            }
            "/sessions" => {
                let pm = PersistenceManager::new();
                let sessions = pm.list_sessions()?;
                for s in sessions {
                    println!("{}", s);
                }
            }
            "/resume" => {
                if cmd_args.is_empty() {
                    println!("Usage: /resume <session_id>");
                } else {
                    let pm = PersistenceManager::new();
                    if let Some(snapshot) = pm.load_session(cmd_args)? {
                        let mut session = Session::new(self.config.clone());
                        session.initialize().await?;
                        session.session_id = snapshot.session_id;
                        session.created_at = snapshot.created_at;
                        session.updated_at = snapshot.updated_at;
                        session.turn_count = snapshot.turn_count;
                        session.context_manager.total_usage = snapshot.total_usage;
                        for msg in snapshot.messages {
                            match msg.get("role").and_then(|v| v.as_str()).unwrap_or("") {
                                "system" => {}
                                "user" => session.context_manager.add_user_message(
                                    msg.get("content").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                                ),
                                "assistant" => session.context_manager.add_assistant_message(
                                    msg.get("content").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                                    msg.get("tool_calls").cloned(),
                                ),
                                "tool" => session.context_manager.add_tool_result(
                                    msg.get("tool_call_id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                                    msg.get("content").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                                ),
                                _ => {}
                            }
                        }
                        agent.session = session;
                        println!("Resumed session: {}", agent.session.session_id);
                    } else {
                        println!("Session does not exist");
                    }
                }
            }
            "/checkpoint" => {
                let pm = PersistenceManager::new();
                let snap = SessionSnapshot {
                    session_id: agent.session.session_id.clone(),
                    created_at: agent.session.created_at,
                    updated_at: agent.session.updated_at,
                    turn_count: agent.session.turn_count,
                    messages: agent.session.context_manager.get_messages(),
                    total_usage: agent.session.context_manager.total_usage,
                };
                let checkpoint_id = pm.save_checkpoint(&snap)?;
                println!("Checkpoint created: {}", checkpoint_id);
            }
            "/checkpoints" => {
                let pm = PersistenceManager::new();
                for cp in pm.list_checkpoints()? {
                    println!("{}", cp);
                }
            }
            "/restore" => {
                if cmd_args.is_empty() {
                    println!("Usage: /restore <checkpoint_id>");
                } else {
                    let pm = PersistenceManager::new();
                    if let Some(snapshot) = pm.load_checkpoint(cmd_args)? {
                        let mut session = Session::new(self.config.clone());
                        session.initialize().await?;
                        session.session_id = snapshot.session_id;
                        session.created_at = snapshot.created_at;
                        session.updated_at = snapshot.updated_at;
                        session.turn_count = snapshot.turn_count;
                        session.context_manager.total_usage = snapshot.total_usage;
                        for msg in snapshot.messages {
                            match msg.get("role").and_then(|v| v.as_str()).unwrap_or("") {
                                "system" => {}
                                "user" => session.context_manager.add_user_message(
                                    msg.get("content").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                                ),
                                "assistant" => session.context_manager.add_assistant_message(
                                    msg.get("content").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                                    msg.get("tool_calls").cloned(),
                                ),
                                "tool" => session.context_manager.add_tool_result(
                                    msg.get("tool_call_id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                                    msg.get("content").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                                ),
                                _ => {}
                            }
                        }
                        agent.session = session;
                        println!("Resumed session: {}", agent.session.session_id);
                    } else {
                        println!("Checkpoint does not exist");
                    }
                }
            }
            _ => println!("Unknown command: {}", cmd_name),
        }
        Ok(true)
    }
}
