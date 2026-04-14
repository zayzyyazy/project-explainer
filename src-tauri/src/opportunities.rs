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

pub const OPPORTUNITY_SYSTEM_PROMPT: &str = r#"You help a solo builder package an EXISTING project—not invent a new company. Year: 2026. Write like a builder making a decision, not like a consultant writing a report.

STRICT OUTPUT RULES:
- Output a SINGLE JSON object only. No text before or after it.
- Do NOT wrap the JSON in markdown code fences. Do NOT use ``` anywhere.
- Do NOT include markdown headings, bold, or lists outside JSON.

INPUT:
The user message JSON includes "existing_project_analysis"—that object is the ONLY ground truth. Do not invent products, features, or systems that are not implied by that analysis.

WHAT COUNTS AS A VALID OPPORTUNITY:
Each opportunity MUST be one of: (a) an extension of the existing project, (b) a repackaging of what it already does, or (c) a niche application of its existing functionality.
- Do NOT invent unrelated tools or platforms.
- Do NOT suggest ideas that require building a completely new system from scratch.
- You are not brainstorming greenfield startups. You are packaging what already exists (or a thin extension of it).

HARD CONSTRAINTS:
- Each opportunity MUST directly reuse or extend the existing project as described in the analysis.
- If an idea cannot realistically be sold or seriously tested by a solo builder within 7 days (offer + outreach + demo), it is INVALID—discard it and replace with a tighter idea.
- Choose ONE specific buyer per opportunity. If multiple audiences are possible, pick the single most realistic buyer and ignore the rest—no multi-audience or "many industries" language.

LENGTH LIMITS (enforce strictly; short lines only):
- title: short headline
- what_it_is: at most 3 lines
- problem: at most 2 lines
- why_this_problem_is_real_now: at most 2 lines
- target_customer: exactly 1 line, very specific (role + context)
- who_exactly_to_contact: short—specific role/title and where to find them, not an essay
- how_to_package: at most 2 lines
- pricing_logic: at most 2 lines
- distribution_strategy: array of SHORT strings (no long paragraphs; prefer 2–4 tight bullets total across the array, not a wall of text)
- first_3_steps_to_validate: EXACTLY 3 strings—each a short bullet. Must be executable immediately: post, DM, demo, landing page, call, email a named type of prospect—NOT vague "research the market" or "validate demand" without a concrete action.
- risk_level: short (e.g. low/medium/high + few words)
- why_this_could_fail: at most 2 lines

STYLE:
- No generic startup fluff, no "AI SaaS for everyone," no broad platforms.
- Brutally honest in why_this_could_fail.

Each opportunity must include ALL of these fields (snake_case):
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
