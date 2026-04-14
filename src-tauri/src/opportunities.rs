use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::claude::{extract_json_text, trim_json_string_values};

//
// ───────────────────────────────────────────────────────────
// V2 ONLY — separate from AnalysisPayload
// ───────────────────────────────────────────────────────────
//

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct Opportunity {
    pub title: String,
    pub what_it_is: String,
    pub problem: String,
    pub why_this_problem_is_real_now: String,
    pub target_customer: String,
    pub who_exactly_to_contact: String,
    pub how_to_package: String,
    pub pricing_logic: String,
    pub distribution_strategy: Vec<String>,
    pub first_3_steps_to_validate: Vec<String>,
    pub risk_level: String,
    pub why_this_could_fail: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct OpportunityPayload {
    pub opportunities: Vec<Opportunity>,
}

pub const OPPORTUNITY_SYSTEM_PROMPT: &str = r#"You are "Packaging Mode" — you help a solo builder monetize what is ALREADY in the repo. Year: 2026. Voice: terse, unsentimental, zero startup poetry.

STRICT OUTPUT RULES:
- Output a SINGLE JSON object only. No text before or after it.
- Do NOT wrap the JSON in markdown code fences. Do NOT use ``` anywhere.
- Do NOT include markdown headings, bold, or lists outside JSON.

INPUT:
The user message JSON includes "existing_project_analysis" — that object is the ONLY ground truth. If you invent a product name not implied by the analysis, you failed.

VALID OPPORTUNITY = ONLY:
A) Ship a thin extension of this codebase, OR
B) Repackage the same capability for one niche, OR
C) Apply existing functionality to one buyer you can name.

INVALID:
- Greenfield platforms, "AI wrappers for everyone," unrelated tools, anything needing 6 months of build before first dollar.

HARD RULES:
- Each opportunity MUST trace to concrete capabilities in the analysis (stack, features, flows).
- If it cannot be pitched or smoke-tested by one person in ~7 days, replace it.
- ONE buyer only. Pick the most plausible; delete the rest of your imagination.
- Ban words: "ecosystem," "synergy," "paradigm," "transform industries," "unlock value."

LENGTH LIMITS (strict):
- title: ≤12 words
- what_it_is: ≤3 lines
- problem: ≤2 lines
- why_this_problem_is_real_now: ≤2 lines
- target_customer: 1 line, role + context (e.g. "solo Shopify dev in EU")
- who_exactly_to_contact: ≤3 short phrases
- how_to_package: ≤2 lines
- pricing_logic: ≤2 lines
- distribution_strategy: 2–4 SHORT strings total
- first_3_steps_to_validate: EXACTLY 3 strings—each a concrete next action (DM, post, call, demo, landing page). No "do market research."
- risk_level: one short line
- why_this_could_fail: ≤2 lines, honest

Each opportunity must include ALL fields (snake_case):
title, what_it_is, problem, why_this_problem_is_real_now, target_customer, who_exactly_to_contact, how_to_package, pricing_logic,
distribution_strategy (array of strings),
first_3_steps_to_validate (exactly 3 strings),
risk_level,
why_this_could_fail

Produce exactly 3 to 5 opportunities.

Top-level JSON shape (required):
{"opportunities":[{...}, ...]}

Return only the JSON object."#;

pub fn build_opportunity_user_message(analysis: &serde_json::Value) -> String {
    json!({
        "task": "generate_sellable_opportunities",
        "existing_project_analysis": analysis,
    })
    .to_string()
}

fn normalize_opportunity_object(v: &mut serde_json::Value) {
    if let Some(obj) = v.as_object_mut() {
        for key in ["distribution_strategy", "first_3_steps_to_validate"] {
            if let Some(val) = obj.get_mut(key) {
                if val.is_string() {
                    let s = val.as_str().unwrap_or("").to_string();
                    *val = json!([s]);
                }
            }
        }
    }
}

pub fn parse_and_validate_opportunities(json_str: &str) -> Result<OpportunityPayload, String> {
    let cleaned = extract_json_text(json_str);

    let mut v: serde_json::Value = serde_json::from_str(&cleaned).map_err(|e| {
        format!(
            "Opportunity model response was not valid JSON: {}",
            e
        )
    })?;

    if let Some(arr) = v.get_mut("opportunities").and_then(|x| x.as_array_mut()) {
        for item in arr.iter_mut() {
            normalize_opportunity_object(item);
        }
    } else {
        return Err("Missing top-level \"opportunities\" array".into());
    }

    trim_json_string_values(&mut v);

    let parsed: OpportunityPayload =
        serde_json::from_value(v).map_err(|e| format!("Opportunity schema error: {}", e))?;

    let n = parsed.opportunities.len();
    if n < 3 || n > 5 {
        return Err(format!(
            "Expected 3–5 opportunities, got {}",
            n
        ));
    }

    for (i, o) in parsed.opportunities.iter().enumerate() {
        if o.title.trim().is_empty() {
            return Err(format!("Opportunity {}: title must be non-empty", i + 1));
        }
        if o.first_3_steps_to_validate.len() != 3 {
            return Err(format!(
                "Opportunity \"{}\": first_3_steps_to_validate must have exactly 3 items",
                o.title
            ));
        }
    }

    Ok(parsed)
}
