use crate::tools::base::Tool;

pub fn get_system_prompt(
    developer_instructions: Option<&str>,
    user_instructions: Option<&str>,
    memory: Option<&str>,
    tools: Vec<&dyn Tool>,
) -> String {
    [
        get_identity_section(),
        get_environment_section(),
        get_shell_info(),
        get_agents_md_section(),
        get_security_section(),
        get_operational_section(),
        get_developer_instructions_section(developer_instructions.unwrap_or("")),
        get_user_instructions_section(user_instructions.unwrap_or("")),
        get_memory_section(memory.unwrap_or("")),
        get_tool_guidelines_section(tools),
    ]
    .join("\n\n")
}

pub fn get_identity_section() -> String {
    "You are Codex, a coding agent based on GPT-5.".to_string()
}

pub fn get_environment_section() -> String {
    "Environment: terminal coding workflow.".to_string()
}

pub fn get_shell_info() -> String {
    "Shell: platform default shell.".to_string()
}

pub fn get_agents_md_section() -> String {
    "Read AGENTS.md and follow project instructions.".to_string()
}

pub fn get_security_section() -> String {
    "Do not run destructive commands without approval.".to_string()
}

pub fn get_operational_section() -> String {
    "Use tools, verify changes, and report outcomes clearly.".to_string()
}

pub fn get_developer_instructions_section(instructions: &str) -> String {
    format!("Developer instructions:\n{}", instructions)
}

pub fn get_user_instructions_section(instructions: &str) -> String {
    format!("User instructions:\n{}", instructions)
}

pub fn get_memory_section(memory: &str) -> String {
    format!("Memory:\n{}", memory)
}

pub fn get_tool_guidelines_section(tools: Vec<&dyn Tool>) -> String {
    let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
    format!("Tools: {}", names.join(", "))
}

pub fn get_compression_prompt() -> String {
    "Compress the conversation while preserving actionable details.".to_string()
}

pub fn create_loop_breaker_prompt(loop_description: &str) -> String {
    format!(
        "Loop detected: {}. Try a different strategy and avoid repeating the same calls.",
        loop_description
    )
}
