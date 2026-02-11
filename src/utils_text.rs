pub fn get_tokenizer(_model: &str) -> &'static str {
    "simple"
}

pub fn count_tokens(text: &str, _model: &str) -> usize {
    text.split_whitespace().count()
}

pub fn estimate_tokens(text: &str) -> usize {
    count_tokens(text, "gpt-4")
}

pub fn truncate_text(text: &str, target_tokens: usize, suffix: &str, model: &str) -> String {
    if count_tokens(text, model) <= target_tokens {
        return text.to_string();
    }
    truncate_by_lines(text, target_tokens, suffix, model)
}

pub fn truncate_by_lines(text: &str, target_tokens: usize, suffix: &str, model: &str) -> String {
    let mut out = Vec::new();
    for line in text.lines() {
        out.push(line);
        if count_tokens(&out.join("\n"), model) >= target_tokens {
            break;
        }
    }
    let mut joined = out.join("\n");
    joined.push_str(suffix);
    joined
}

pub fn truncate_by_chars(text: &str, target_tokens: usize, suffix: &str, model: &str) -> String {
    let approx_chars = target_tokens * 4;
    if text.len() <= approx_chars {
        return text.to_string();
    }
    let mut s = text[..approx_chars].to_string();
    while count_tokens(&s, model) > target_tokens && !s.is_empty() {
        s.pop();
    }
    s.push_str(suffix);
    s
}
