use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::claude::extract_json_text;

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

pub const OPPORTUNITY_SYSTEM_PROMPT: &str = r#"You are a pragmatic product strategist helping a solo builder turn an EXISTING codebase analysis into sellable opportunities. Year: 2026.

STRICT OUTPUT RULES:
- Output a SINGLE JSON object only. No text before or after it.
- Do NOT wrap the JSON in markdown code fences. Do NOT use ``` anywhere.
- Do NOT include markdown headings, bold, or lists outside JSON.

INPUT:
You will receive a JSON user message with key "existing_project_analysis" containing the stored analysis of a real project (features, stack, product intelligence, etc.). Treat that object as the only ground truth.

TASK:
Produce exactly 3 to 5 opportunities. Each must be:
- Grounded in that analysis (reference concrete capabilities, stack, or problems implied by the analysis—not generic "AI SaaS").
- Narrow and specific: one clear buyer persona and one clear packaged offer per opportunity.
- Realistic for 2026: name plausible channels, buyer titles, and validation steps someone could run in days or weeks—not fantasy VC-scale platforms.
- Brutally honest: no hype, no "useful for many industries," no "platform for everyone."
- If the underlying project is vague or weak, say so inside "why_this_could_fail" and/or sharpen into a niche—or explain what would make the opportunity stronger.

Each opportunity must include ALL of these string/array fields (snake_case keys at the opportunity level):
title, what_it_is, problem, why_this_problem_is_real_now, target_customer, who_exactly_to_contact, how_to_package, pricing_logic,
distribution_strategy (array of short strings),
first_3_steps_to_validate (exactly 3 short strings: concrete next actions),
risk_level (e.g. low/medium/high with a word of context),
why_this_could_fail

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
