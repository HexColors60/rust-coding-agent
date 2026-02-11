use anyhow::Result;

#[derive(Default)]
pub struct ChatCompactor;

impl ChatCompactor {
    pub fn new() -> Self {
        Self
    }

    pub async fn compact(&self, messages: &[serde_json::Value], target_tokens: usize) -> Result<String> {
        let mut joined = String::new();
        for msg in messages {
            joined.push_str(&serde_json::to_string(msg)?);
            joined.push('\n');
            if joined.len() > target_tokens * 4 {
                break;
            }
        }
        Ok(joined)
    }
}
