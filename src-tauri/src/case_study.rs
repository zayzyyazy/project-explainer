use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::claude::{extract_json_text, trim_json_string_values};

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct CaseStudyPayload {
    pub title: String,
    pub problem: String,
    pub solution: String,
    pub outcome: String,
    /// How outcome was derived: e.g. "inferred_from_codebase" or "signals_only_no_client_metrics"
    pub outcome_basis: String,
    pub narrative: String,
    pub linkedin_hook: String,
    pub quote_ready_one_liner: String,
    pub what_we_built: Vec<String>,
}

pub const CASE_STUDY_SYSTEM_PROMPT: &str = r#"You are "Story & Code" — a sharp freelance positioning writer. Your ONLY job: turn a stored project analysis JSON into ONE client-winning case study a developer can paste into proposals, portfolios, or LinkedIn.

VOICE: Direct, confident, specific. Never corporate filler. Never "leverage synergies." Never invent client names or dollar metrics.

STRICT OUTPUT RULES:
- Output a SINGLE JSON object only. No text before or after it.
- Do NOT wrap the JSON in markdown code fences. Do NOT use ``` anywhere.

GROUNDING:
- The user message includes "existing_project_analysis" — that is the sole source of truth for what was built.
- Describe problems and outcomes ONLY in ways supported by that analysis (and reasonable inference from product_intent, problem_it_solves, core_features, architecture).
- Do NOT claim revenue, user counts, ROI %, or client quotes unless they appear verbatim in the analysis. If impact is inferred, say so plainly in outcome_basis and keep outcome language honest (e.g. "reduced manual steps", "faster feedback loop") not fake numbers.

CASE STUDY JOB:
- Reframe technical work as BUSINESS value: time saved, risk reduced, clarity gained, faster shipping — grounded in what the repo actually does.
- outcome: 2–4 short sentences max. If no metrics exist, write plausible qualitative impact and mark outcome_basis accordingly.
- narrative: one cohesive story (Problem → what we built → result), 120–220 words, scannable, no bullet list inside this string.
- what_we_built: 3–6 tight bullets; each references real capability implied by the analysis (not fantasy features).

LENGTH:
- title: punchy headline, under 90 characters if possible.
- linkedin_hook: exactly 2 short sentences (under 280 chars total).
- quote_ready_one_liner: one sentence, speakable aloud.

Required JSON keys (exact snake_case):
title, problem, solution, outcome, outcome_basis, narrative, linkedin_hook, quote_ready_one_liner, what_we_built (array of strings)

Return only the JSON object."#;

pub fn build_case_study_user_message(analysis: &serde_json::Value) -> String {
    json!({
        "task": "generate_client_case_study",
        "existing_project_analysis": analysis,
    })
    .to_string()
}

fn normalize_what_we_built(v: &mut serde_json::Value) {
    if let Some(obj) = v.as_object_mut() {
        if let Some(val) = obj.get_mut("what_we_built") {
            if val.is_string() {
                let s = val.as_str().unwrap_or("").to_string();
                *val = json!([s]);
            }
        }
    }
}

pub fn parse_and_validate_case_study(json_str: &str) -> Result<CaseStudyPayload, String> {
    let cleaned = extract_json_text(json_str);

    let mut v: serde_json::Value = serde_json::from_str(&cleaned).map_err(|e| {
        format!("Case study response was not valid JSON: {}", e)
    })?;

    normalize_what_we_built(&mut v);
    trim_json_string_values(&mut v);

    let parsed: CaseStudyPayload =
        serde_json::from_value(v).map_err(|e| format!("Case study schema error: {}", e))?;

    if parsed.title.trim().is_empty() {
        return Err("Case study title must be non-empty".into());
    }
    if parsed.problem.trim().is_empty() || parsed.solution.trim().is_empty() {
        return Err("problem and solution must be non-empty".into());
    }
    if parsed.outcome.trim().is_empty() || parsed.narrative.trim().is_empty() {
        return Err("outcome and narrative must be non-empty".into());
    }
    if parsed.what_we_built.is_empty() {
        return Err("what_we_built must not be empty".into());
    }

    Ok(parsed)
}
