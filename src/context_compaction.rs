use anyhow::Result;
use serde_json::Value;

use crate::utils_text::{count_tokens, truncate_text};

#[derive(Default)]
pub struct ChatCompactor;

impl ChatCompactor {
    pub fn new() -> Self {
        Self
    }

    pub async fn compact(&self, messages: &[Value], target_tokens: usize) -> Result<String> {
        let mut blocks = Vec::new();
        for msg in messages {
            let role = msg
                .get("role")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            let content = msg
                .get("content")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();

            // Tool outputs are usually largest; keep them but trim aggressively.
            let content = if role == "tool" {
                truncate_text(
                    &content,
                    (target_tokens / 8).max(100),
                    "\n...[tool output truncated]",
                    "gpt-4",
                )
            } else {
                content
            };
            blocks.push(format!("{}:\n{}", role, content));
        }

        let mut output = blocks.join("\n\n");
        if count_tokens(&output, "gpt-4") > target_tokens {
            output = truncate_text(
                &output,
                target_tokens,
                "\n...[conversation summary truncated]",
                "gpt-4",
            );
        }
        Ok(output)
    }
}
