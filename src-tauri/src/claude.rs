use crate::scanner::{FileIndexEntry, ScanResult, SelectedFile};
use serde::{Deserialize, Serialize};
use serde_json::json;

//
// ───────────────────────────────────────────────────────────
// DATA STRUCTURES
// ───────────────────────────────────────────────────────────
//

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct ImportantFile {
    pub path: String,
    pub why_it_matters: String,
    pub confidence_notes: String,
    pub possible_gaps_or_uncertainties: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct GoToMarket {
    pub target_user: String,
    pub sell_as: String,
    pub where_to_sell: Vec<String>,
    pub first_steps: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct ProductIntelligence {
    pub category: String,
    pub target_users: Vec<String>,
    pub use_cases: Vec<String>,
    pub monetization_models: Vec<String>,
    pub distribution_channels: Vec<String>,
    pub product_stage: String,
    pub what_is_missing: Vec<String>,
    pub strengths: Vec<String>,
    pub risks: Vec<String>,
    pub go_to_market: GoToMarket,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct AnalysisPayload {
    pub project_name: String,
    pub project_intent: String,
    pub when_built: String,
    pub one_line_summary: String,
    pub deep_explanation: String,
    pub full_narrative_explanation: String,
    pub problem_it_solves: String,
    pub why_it_matters: String,
    pub core_features: Vec<String>,
    pub key_flows: Vec<String>,
    pub tech_stack: Vec<String>,
    pub architecture_overview: String,
    pub how_it_works_step_by_step: Vec<String>,
    pub design_decisions: Vec<String>,
    pub tradeoffs_and_limitations: Vec<String>,
    pub how_to_run: String,
    pub example_outputs: Vec<String>,
    pub important_files: Vec<ImportantFile>,
    pub product_intelligence: ProductIntelligence,
}

//
// ───────────────────────────────────────────────────────────
// BUILD USER MESSAGE
// ───────────────────────────────────────────────────────────
//

pub fn build_user_message(
    folder_name: &str,
    scan: &ScanResult,
    stack_hint: &[String],
) -> String {
    let index_lines: Vec<String> = scan
        .file_index
        .iter()
        .map(|e: &FileIndexEntry| {
            format!("{} | ext:{} | {} bytes", e.rel_path, e.extension, e.size)
        })
        .collect();

    let files_json: Vec<serde_json::Value> = scan
        .selected_files
        .iter()
        .map(|f: &SelectedFile| {
            json!({
                "path": f.rel_path,
                "content": f.content,
            })
        })
        .collect();

    json!({
        "analysis_mode": "portfolio_deep_explanation",
        "folder_name": folder_name,
        "detected_stack_signals": stack_hint,
        "file_index_truncated": scan.index_truncated,
        "indexed_files": index_lines,
        "selected_file_contents": files_json.into_iter().take(5).collect::<Vec<_>>(),
    })
    .to_string()
}

//
// ───────────────────────────────────────────────────────────
// CLEAN JSON RESPONSE
// ───────────────────────────────────────────────────────────
//

pub fn extract_json_text(raw: &str) -> String {
    let s = raw.trim();

    if let Some(pos) = s.find("```") {
        let mut inner = &s[pos + 3..];
        inner = inner.trim_start();

        if inner.starts_with("json") {
            inner = &inner[4..];
        }

        inner = inner.trim_start();

        if let Some(end) = inner.rfind("```") {
            return inner[..end].trim().to_string();
        }

        return inner.trim().to_string();
    }

    s.to_string()
}

/// Trims leading/trailing whitespace on every JSON string value (recursive). Structure and keys unchanged.
pub fn trim_json_string_values(v: &mut serde_json::Value) {
    match v {
        serde_json::Value::Object(map) => {
            for val in map.values_mut() {
                trim_json_string_values(val);
            }
        }
        serde_json::Value::Array(arr) => {
            for item in arr.iter_mut() {
                trim_json_string_values(item);
            }
        }
        serde_json::Value::String(s) => {
            *s = s.trim().to_string();
        }
        _ => {}
    }
}

//
// ───────────────────────────────────────────────────────────
// PARSE + VALIDATE
// ───────────────────────────────────────────────────────────
//

fn normalize_str_to_array(v: &mut serde_json::Value, key: &str) {
    if let Some(obj) = v.as_object_mut() {
        if let Some(val) = obj.get(key).cloned() {
            if val.is_string() {
                let s = val.as_str().unwrap_or("").to_string();
                obj.insert(key.to_string(), json!([s]));
            }
        }
    }
}

pub fn parse_and_validate(json_str: &str) -> Result<AnalysisPayload, String> {
    let cleaned = extract_json_text(json_str);

    let mut v: serde_json::Value = serde_json::from_str(&cleaned).map_err(|e| {
        format!(
            "Model response was not valid JSON (no fallback allowed): {}",
            e
        )
    })?;

    let top_level_arrays = [
        "core_features",
        "key_flows",
        "tech_stack",
        "how_it_works_step_by_step",
        "design_decisions",
        "tradeoffs_and_limitations",
        "example_outputs",
    ];

    for f in top_level_arrays {
        normalize_str_to_array(&mut v, f);
    }

    if let Some(pi) = v.get_mut("product_intelligence") {
        if let Some(pi_obj) = pi.as_object_mut() {
            for field in [
                "target_users",
                "use_cases",
                "monetization_models",
                "distribution_channels",
                "what_is_missing",
                "strengths",
                "risks",
            ] {
                if let Some(val) = pi_obj.get_mut(field) {
                    if val.is_string() {
                        let s = val.as_str().unwrap_or("").to_string();
                        *val = json!([s]);
                    }
                }
            }

            if let Some(gtm) = pi_obj.get_mut("go_to_market") {
                if let Some(gtm_obj) = gtm.as_object_mut() {
                    for field in ["where_to_sell", "first_steps"] {
                        if let Some(val) = gtm_obj.get_mut(field) {
                            if val.is_string() {
                                let s = val.as_str().unwrap_or("").to_string();
                                *val = json!([s]);
                            }
                        }
                    }
                }
            }
        }
    }

    trim_json_string_values(&mut v);

    let parsed: AnalysisPayload =
        serde_json::from_value(v).map_err(|e| format!("Schema error: {}", e))?;

    if parsed.full_narrative_explanation.trim().is_empty() {
        return Err("full_narrative_explanation must be non-empty".into());
    }

    if parsed.core_features.is_empty() || parsed.tech_stack.is_empty() {
        return Err("core_features and tech_stack must not be empty".into());
    }

    if parsed.important_files.is_empty() {
        return Err("important_files must not be empty".into());
    }

    Ok(parsed)
}

//
// ───────────────────────────────────────────────────────────
// SYSTEM PROMPT — JSON ONLY
// ───────────────────────────────────────────────────────────
//

pub const ANALYSIS_SYSTEM_PROMPT: &str = r#"You are the "Repo Interpreter" — forensic, plain-spoken, allergic to bullshit. You read a codebase snapshot in the user message and produce structured truth for a working developer, not a slide deck.

STRICT OUTPUT RULES:
- Output a SINGLE JSON object only. No text before or after it.
- Do NOT wrap the JSON in markdown code fences. Do NOT use ``` anywhere.
- Do NOT include markdown headings, bold, or lists outside JSON. All prose must be INSIDE JSON string fields.

GROUNDING (NON-NEGOTIABLE):
- Do NOT invent features, modules, integrations, or behaviors not evidenced by the provided files or clear file/index patterns.
- Tie claims to evidence: paths, filenames, configs, stack hints, visible code patterns. If you cannot cite a reason, say "unknown" or park doubt in confidence_notes / possible_gaps_or_uncertainties.
- Forbidden phrases: "many industries," "any team," "enterprise-grade," "AI-powered platform for everyone," "revolutionize," "game-changer."

VERBOSITY:
- Short fields stay SHORT: one_line_summary, problem_it_solves, why_it_matters, architecture_overview — each sentence earns its place.
- deep_explanation: max ~6–8 lines, no fluff. Say what runs, how pieces connect, what is surprising.
- full_narrative_explanation: still required and non-empty; keep it useful (not a novel). Prefer clarity over breadth.
- Arrays: bullets under 15 words each unless a technical term needs it.

PRODUCT INTELLIGENCE:
- Must read ONLY from this repo’s reality. If the product is a CLI tool, do not describe it as a B2B SaaS suite. Match category, stage, and buyers to what the code actually enables.
- No "you could also build a marketplace" unless the repo already contains that domain.

Required top-level keys (exact names, snake_case):
project_name, project_intent, when_built, one_line_summary, deep_explanation, full_narrative_explanation
problem_it_solves, why_it_matters
core_features, key_flows, tech_stack (arrays of strings)
architecture_overview
how_it_works_step_by_step, design_decisions, tradeoffs_and_limitations, example_outputs (arrays of strings)
how_to_run (string)
important_files: array of objects with path, why_it_matters, confidence_notes, possible_gaps_or_uncertainties
product_intelligence: object with category, target_users, use_cases, monetization_models, distribution_channels, product_stage, what_is_missing, strengths, risks (arrays where listed), and go_to_market: { target_user, sell_as, where_to_sell, first_steps }

Return only the JSON object."#;

//
// ───────────────────────────────────────────────────────────
// CLAUDE API
// ───────────────────────────────────────────────────────────
//

pub fn call_claude(api_key: &str, model: &str, user_message: &str) -> Result<String, String> {
    call_claude_with_system(api_key, model, ANALYSIS_SYSTEM_PROMPT, user_message)
}

/// Same transport as `call_claude`, but accepts a custom system prompt
/// so V2 features can use a different schema without touching V1.
pub fn call_claude_with_system(
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

    let strict_user_message = format!(
        "Return ONLY a valid JSON object. No markdown, no explanation.\n\n{}",
        user_message
    );

    let body = json!({
        "model": model,
        "max_tokens": 16384,
        "temperature": 0,
        "system": system_prompt,
        "messages": [
            {
                "role": "user",
                "content": [
                    {
                        "type": "text",
                        "text": strict_user_message
                    }
                ]
            }
        ]
    });

    let res = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", key)
        .header("anthropic-version", "2023-06-01")
        .json(&body)
        .send()
        .map_err(|e| format!("Network error: {}", e))?;

    let status = res.status();
    let raw = res.text().unwrap_or_default();

    if !status.is_success() {
        return Err(format!("Claude API error: {}", raw));
    }

    let v: serde_json::Value = serde_json::from_str(&raw)
        .map_err(|e| format!("Claude HTTP envelope was not JSON: {}", e))?;

    let text = v
        .pointer("/content/0/text")
        .and_then(|x| x.as_str())
        .ok_or_else(|| format!("Unexpected Claude response shape: {}", raw))?;

    if !text.trim().starts_with("{") {
        return Err(format!("Claude returned non-JSON text:\n{}", text));
    }

    Ok(text.to_string())
}