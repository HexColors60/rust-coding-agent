use serde_json::Value;

pub(crate) fn parse_sse_events(text: &str) -> Vec<Value> {
    let mut out = Vec::new();
    let mut current_data = String::new();

    for raw_line in text.lines() {
        let line = raw_line.trim_end_matches('\r');
        if line.is_empty() {
            if !current_data.trim().is_empty() {
                if let Ok(v) = serde_json::from_str::<Value>(current_data.trim()) {
                    out.push(v);
                }
            }
            current_data.clear();
            continue;
        }
        if line.starts_with(':') || line.starts_with("event:") || line.starts_with("id:") {
            continue;
        }
        if let Some(data) = line.strip_prefix("data:") {
            if !current_data.is_empty() {
                current_data.push('\n');
            }
            current_data.push_str(data.trim());
        }
    }

    if !current_data.trim().is_empty() {
        if let Ok(v) = serde_json::from_str::<Value>(current_data.trim()) {
            out.push(v);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::parse_sse_events;
    use serde_json::json;

    #[test]
    fn parse_sse_events_single_and_multiline_data() {
        let fixture = "event: message\n\
id: 1\n\
data: {\"jsonrpc\":\"2.0\",\"id\":1,\n\
data: \"result\":{\"ok\":true}}\n\
\n\
data: {\"jsonrpc\":\"2.0\",\"id\":2,\"result\":{\"tools\":[]}}\n\
\n";

        let parsed = parse_sse_events(fixture);
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0]["id"], json!(1));
        assert_eq!(parsed[1]["id"], json!(2));
    }

    #[test]
    fn parse_sse_events_ignores_non_json_payloads() {
        let fixture = "data: not-json\n\n\
data: {\"jsonrpc\":\"2.0\",\"id\":3,\"result\":{\"x\":1}}\n\n";
        let parsed = parse_sse_events(fixture);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0]["id"], json!(3));
    }
}
