use serde_json::Value;

pub fn get_console() {}

pub struct Tui;

impl Tui {
    pub fn new() -> Self {
        Self
    }

    pub fn print_welcome(&self, title: &str, lines: &[String]) {
        println!("== {} ==", title);
        for line in lines {
            println!("{}", line);
        }
    }

    pub fn begin_assistant(&self) {
        print!("\n[assistant]> ");
    }

    pub fn end_assistant(&self) {
        println!();
    }

    pub fn stream_assistant_delta(&self, content: &str) {
        print!("{}", content);
    }

    pub fn tool_call_start(&self, call_id: &str, name: &str, args: &Value) {
        println!("\n[tool:start] {} #{} args={}", name, call_id, args);
    }

    pub fn tool_call_complete(
        &self,
        call_id: &str,
        name: &str,
        success: bool,
        output: &str,
        error: Option<&str>,
    ) {
        println!(
            "\n[tool:done] {} #{} success={} output={}",
            name, call_id, success, output
        );
        if let Some(e) = error {
            println!("[tool:error] {}", e);
        }
    }

    pub fn handle_confirmation(&self, _description: &str) -> bool {
        false
    }

    pub fn show_help(&self) {
        println!(
            "/help /exit /quit /clear /config /model /approval /stats /tools /mcp /save /checkpoint /checkpoints /restore /sessions /resume"
        );
    }
}
