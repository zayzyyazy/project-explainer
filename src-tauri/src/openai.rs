use crate::claude::ANALYSIS_SYSTEM_PROMPT;
use serde_json::{json, Value};

fn map_openai_http_error(status: u16, body: &str) -> String {
    let lower = body.to_lowercase();
    match status {
        401 | 403 => "API request failed: invalid or revoked API key. Check Settings.".into(),
        402 | 429 => {
            "Provider may be out of credits or unavailable. Try again later or switch provider in Settings.".into()
        }
        _ if lower.contains("insufficient_quota")
            || lower.contains("billing_hard")
            || lower.contains("credit") =>
        {
            "Provider may be out of credits or unavailable. Try again later or switch provider in Settings.".into()
        }
        _ => format!(
            "API request failed (HTTP {}). Provider may be out of credits or unavailable.",
            status
        ),
    }
}

pub fn call_openai(api_key: &str, model: &str, user_message: &str) -> Result<String, String> {
    call_openai_with_system(api_key, model, ANALYSIS_SYSTEM_PROMPT, user_message)
}

pub fn call_openai_with_system(
    api_key: &str,
    model: &str,
    system_prompt: &str,
    user_message: &str,
) -> Result<String, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| e.to_string())?;

    let key = api_key.trim();
    if key.is_empty() {
        return Err("API key missing. Add your key in Settings or set the provider API key in the environment.".into());
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
        .map_err(|e| format!("API request failed: {} (check network).", e))?;

    let status = res.status();
    let raw = res.text().unwrap_or_default();

    if !status.is_success() {
        return Err(map_openai_http_error(status.as_u16(), &raw));
    }

    let v: Value = serde_json::from_str(&raw)
        .map_err(|e| format!("OpenAI HTTP envelope was not JSON: {}", e))?;

    let text = v["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| format!("Unexpected OpenAI response shape: {}", raw))?;

    Ok(text.to_string())
}