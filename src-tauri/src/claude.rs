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
    /// Brief internal intent / build context (not the main user-facing story).
    pub project_intent: String,
    pub when_built: String,
    pub one_line_summary: String,
    /// Technical / structural depth — keep for collapsible “deep” section only.
    pub deep_explanation: String,
    pub full_narrative_explanation: String,
    pub problem_it_solves: String,
    pub why_it_matters: String,
    /// Non-technical: what happens when someone uses it, workflow replaced, user experience.
    #[serde(default)]
    pub what_it_actually_does: String,
    pub core_features: Vec<String>,
    pub key_flows: Vec<String>,
    /// Stack hints only — short tokens; do not let this dominate the narrative.
    pub tech_stack: Vec<String>,
    pub architecture_overview: String,
    pub how_it_works_step_by_step: Vec<String>,
    pub design_decisions: Vec<String>,
    pub tradeoffs_and_limitations: Vec<String>,
    pub how_to_run: String,
    pub example_outputs: Vec<String>,
    pub important_files: Vec<ImportantFile>,
    pub product_intelligence: ProductIntelligence,
    /// One line, e.g. “human-in-the-loop AI ops tool” — portfolio category.
    #[serde(default)]
    pub positioning_label: String,
    /// 3–5 bullets, natural interview lines (newline-separated or single string with bullets).
    #[serde(default)]
    pub interview_talking_points: String,
    /// 2–4 sentences: how to present on portfolio / resume.
    #[serde(default)]
    pub portfolio_positioning: String,
    /// Exactly 3 short strings: angle problem / angle solution / angle insight for posts.
    #[serde(default)]
    pub social_content_angles: Vec<String>,
    /// Optional draft post.
    #[serde(default)]
    pub suggested_social_post: String,
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
        "analysis_mode": "readme_first_v2",
        "folder_name": folder_name,
        "readme_present": scan.readme_present,
        "scan_notes": scan.scan_notes,
        "detected_stack_signals": stack_hint,
        "file_index_truncated": scan.index_truncated,
        "indexed_files": index_lines,
        "selected_file_contents": files_json,
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

fn strip_filler_words(s: &str) -> String {
    let banned = [
        "robust",
        "leveraged",
        "seamless",
        "powerful",
        "cutting-edge",
        "cutting edge",
        "game-changer",
        "game changer",
        "unlock value",
        "synergy",
        "paradigm",
        "revolutionary",
        "thrilled",
    ];
    let mut out = s.trim().to_string();
    for b in banned {
        out = out.replace(b, "");
        out = out.replace(&b.to_uppercase(), "");
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn shorten_text(s: &str, max_chars: usize) -> String {
    strip_filler_words(s).chars().take(max_chars).collect::<String>()
}

fn clamp_lines(s: &str, max_lines: usize) -> String {
    s.lines()
        .map(|x| x.trim())
        .filter(|x| !x.is_empty())
        .take(max_lines)
        .collect::<Vec<_>>()
        .join("\n")
}

fn clamp_list(list: &mut Vec<String>, max_items: usize, max_chars: usize) {
    let mut out = Vec::new();
    for item in list.iter().take(max_items) {
        out.push(shorten_text(item, max_chars));
    }
    *list = out;
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
        "social_content_angles",
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

    if let Some(obj) = v.as_object_mut() {
        for (key, default) in [
            ("what_it_actually_does", json!("")),
            ("positioning_label", json!("")),
            ("interview_talking_points", json!("")),
            ("portfolio_positioning", json!("")),
            ("suggested_social_post", json!("")),
        ] {
            obj.entry(key.to_string()).or_insert(default);
        }
        obj.entry("social_content_angles".to_string())
            .or_insert(json!([]));
    }

    let mut parsed: AnalysisPayload =
        serde_json::from_value(v).map_err(|e| format!("Schema error: {}", e))?;

    parsed.one_line_summary = shorten_text(&parsed.one_line_summary, 200);
    parsed.what_it_actually_does = shorten_text(&parsed.what_it_actually_does, 900);
    parsed.positioning_label = shorten_text(&parsed.positioning_label, 120);
    parsed.deep_explanation = shorten_text(&parsed.deep_explanation, 700);
    parsed.problem_it_solves = shorten_text(&parsed.problem_it_solves, 450);
    parsed.why_it_matters = shorten_text(&parsed.why_it_matters, 450);
    parsed.architecture_overview = shorten_text(&parsed.architecture_overview, 500);
    parsed.interview_talking_points = shorten_text(&parsed.interview_talking_points, 1400);
    parsed.portfolio_positioning = shorten_text(&parsed.portfolio_positioning, 650);
    parsed.suggested_social_post = shorten_text(&parsed.suggested_social_post, 800);
    parsed.full_narrative_explanation = clamp_lines(&parsed.full_narrative_explanation, 8);
    clamp_list(&mut parsed.core_features, 5, 160);
    clamp_list(&mut parsed.social_content_angles, 3, 200);
    clamp_list(&mut parsed.key_flows, 5, 120);
    clamp_list(&mut parsed.tech_stack, 8, 48);
    clamp_list(&mut parsed.how_it_works_step_by_step, 5, 120);
    clamp_list(&mut parsed.design_decisions, 4, 120);
    clamp_list(&mut parsed.tradeoffs_and_limitations, 4, 120);
    clamp_list(&mut parsed.example_outputs, 3, 130);

    clamp_list(&mut parsed.product_intelligence.target_users, 3, 100);
    clamp_list(&mut parsed.product_intelligence.use_cases, 3, 100);
    clamp_list(&mut parsed.product_intelligence.monetization_models, 3, 100);
    clamp_list(&mut parsed.product_intelligence.distribution_channels, 3, 100);
    clamp_list(&mut parsed.product_intelligence.what_is_missing, 3, 100);
    clamp_list(&mut parsed.product_intelligence.strengths, 3, 100);
    clamp_list(&mut parsed.product_intelligence.risks, 3, 100);
    clamp_list(
        &mut parsed.product_intelligence.go_to_market.where_to_sell,
        3,
        100,
    );
    clamp_list(
        &mut parsed.product_intelligence.go_to_market.first_steps,
        3,
        100,
    );

    if parsed.full_narrative_explanation.trim().is_empty() {
        return Err("full_narrative_explanation must be non-empty".into());
    }

    if parsed.core_features.is_empty() || parsed.tech_stack.is_empty() {
        return Err("core_features and tech_stack must not be empty".into());
    }

    if parsed.important_files.is_empty() {
        return Err("important_files must not be empty".into());
    }

    if parsed.what_it_actually_does.trim().is_empty() {
        parsed.what_it_actually_does =
            shorten_text(&format!("{} {}", parsed.one_line_summary, parsed.problem_it_solves), 800);
    }
    if parsed.positioning_label.trim().is_empty() {
        parsed.positioning_label =
            shorten_text(parsed.product_intelligence.category.as_str(), 120);
    }
    if parsed.social_content_angles.is_empty() {
        parsed.social_content_angles = vec![
            shorten_text(
                &format!("Problem angle: {}", parsed.problem_it_solves),
                200,
            ),
            shorten_text(
                &format!("Solution angle: {}", parsed.one_line_summary),
                200,
            ),
            shorten_text(
                &format!("Insight angle: {}", parsed.why_it_matters),
                200,
            ),
        ];
    }

    Ok(parsed)
}

//
// ───────────────────────────────────────────────────────────
// SYSTEM PROMPT — JSON ONLY
// ───────────────────────────────────────────────────────────
//

pub const ANALYSIS_SYSTEM_PROMPT: &str = r#"You are a PROJECT INTELLIGENCE analyst. Input: README-first JSON (README + a few configs + shallow file names). Your job is MEANING, VALUE, and POSITIONING — not documentation and not a code walkthrough.

STRICT OUTPUT RULES:
- Output ONE JSON object only. No markdown fences. No prose outside JSON.
- Ground claims in README/configs/index. If unknown, say so in confidence_notes — do not invent features.
- Voice: direct, specific, human. Ban filler: "leverage", "robust", "seamless", "cutting-edge", "game-changer", "thrilled", "unlock value", "synergy", "ecosystem", "paradigm", "revolutionary".

INTELLIGENCE LAYERS (this is the product — prioritize these over stack):

1) one_line_summary — MUST follow this shape: "A [type of system/tool] that [what it does] for [who]". One sentence, sharp.

2) what_it_actually_does — NON-TECHNICAL. What happens when someone uses it, what workflow it replaces or improves, what the user experiences. 4–8 short sentences max, no stack dump.

3) problem_it_solves — CONCRETE: current manual pain, inefficiency, inconsistency, or risk. No generic "teams struggle with communication."

4) why_it_matters — VALUE: time, consistency, risk, decisions, scale — realistic, not exaggerated.

5) core_features — EXACTLY 3 to 5 strings. Each = capability + user-visible value (not "uses React").

6) interview_talking_points — STRING with 3 to 5 bullet lines (start lines with "- "). Confident, natural things to say in an interview. Not a feature list.

7) positioning_label — ONE short phrase: what kind of project is this for a portfolio header (e.g. "local-first portfolio intelligence app", "human-in-the-loop triage workflow").

8) portfolio_positioning — 2–4 sentences: how to frame the project on a portfolio or resume.

TECH / DEEP (secondary — for collapsible "deep" UI only):
- tech_stack: max 8 short tokens, hints only.
- architecture_overview, deep_explanation, full_narrative_explanation, key_flows, how_it_works_step_by_step, design_decisions, tradeoffs: structured, short bullets/lines — NOT long essays. deep_explanation = technical glue + flow at high level.
- project_intent: one line why it was built / scope (internal), not the main story (that is what_it_actually_does).

product_intelligence: map to real use implied by README — category, target_users, use_cases, monetization_models, distribution_channels, product_stage, what_is_missing, strengths, risks, go_to_market { target_user, sell_as, where_to_sell, first_steps }.

important_files: include README first when present; each object: path, why_it_matters, confidence_notes, possible_gaps_or_uncertainties.

Required keys (snake_case, all required):
project_name, project_intent, when_built, one_line_summary, what_it_actually_does, positioning_label
problem_it_solves, why_it_matters
interview_talking_points, portfolio_positioning (strings)
deep_explanation, full_narrative_explanation
core_features, key_flows, tech_stack, architecture_overview
how_it_works_step_by_step, design_decisions, tradeoffs_and_limitations, example_outputs
how_to_run, important_files, product_intelligence

Return only the JSON object."#;

//
// ───────────────────────────────────────────────────────────
// CLAUDE API
// ───────────────────────────────────────────────────────────
//

fn map_anthropic_http_error(status: u16, body: &str) -> String {
    let lower = body.to_lowercase();
    match status {
        401 | 403 => "API request failed: invalid or revoked API key. Check Settings.".into(),
        402 | 429 => {
            "Provider may be out of credits or unavailable. Try again later or switch provider in Settings.".into()
        }
        _ if lower.contains("insufficient_quota")
            || lower.contains("credit")
            || lower.contains("billing") =>
        {
            "Provider may be out of credits or unavailable. Try again later or switch provider in Settings.".into()
        }
        _ => format!(
            "API request failed (HTTP {}). Provider may be out of credits or unavailable.",
            status
        ),
    }
}

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
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| e.to_string())?;

    let key = api_key.trim();
    if key.is_empty() {
        return Err("API key missing. Add your key in Settings or set the provider API key in the environment.".into());
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
        .map_err(|e| format!("API request failed: {} (check network).", e))?;

    let status = res.status();
    let raw = res.text().unwrap_or_default();

    if !status.is_success() {
        return Err(map_anthropic_http_error(status.as_u16(), &raw));
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