use crate::claude::ANALYSIS_SYSTEM_PROMPT;
use serde_json::{json, Value};

pub fn call_openai(api_key: &str, model: &str, user_message: &str) -> Result<String, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .map_err(|e| e.to_string())?;

    let key = api_key.trim();
    if key.is_empty() {
        return Err("API key is empty".into());
    }

    let body = json!({
        "model": model,
        "max_tokens": 16384,
        "temperature": 0.2,
        "messages": [
            { "role": "system", "content": ANALYSIS_SYSTEM_PROMPT },
            { "role": "user", "content": user_message }
        ]
    });

    let res = client
        .post("https://api.openai.com/v1/chat/completions")
        .bearer_auth(key)
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .map_err(|e| format!("Network error: {}", e))?;

    let status = res.status();
    let raw = res.text().unwrap_or_default();

    if !status.is_success() {
        return Err(format!("OpenAI API error: {}", raw));
    }

    let v: Value = serde_json::from_str(&raw)
        .map_err(|e| format!("OpenAI HTTP envelope was not JSON: {}", e))?;

    let text = v["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| format!("Unexpected OpenAI response shape: {}", raw))?;

    Ok(text.to_string())
}

/// Same transport as [`call_openai`], but accepts a custom system prompt (e.g. V2 opportunities).
pub fn call_openai_with_system(
    api_key: &str,
    model: &str,
    system_prompt: &str,
    user_message: &str,
) -> Result<String, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .map_err(|e| e.to_string())?;

    let key = api_key.trim();
    if key.is_empty() {
        return Err("API key is empty".into());
    }

    let body = json!({
        "model": model,
        "max_tokens": 16384,
        "temperature": 0.2,
        "messages": [
            { "role": "system", "content": system_prompt },
            { "role": "user", "content": user_message }
        ]
    });

    let res = client
        .post("https://api.openai.com/v1/chat/completions")
        .bearer_auth(key)
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .map_err(|e| format!("Network error: {}", e))?;

    let status = res.status();
    let raw = res.text().unwrap_or_default();

    if !status.is_success() {
        return Err(format!("OpenAI API error: {}", raw));
    }

    let v: Value = serde_json::from_str(&raw)
        .map_err(|e| format!("OpenAI HTTP envelope was not JSON: {}", e))?;

    let text = v["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| format!("Unexpected OpenAI response shape: {}", raw))?;

    Ok(text.to_string())
}
