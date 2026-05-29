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

pub const OPPORTUNITY_SYSTEM_PROMPT: &str = r#"You are "Packaging Mode" — turn STORED PROJECT INTELLIGENCE into 3–5 realistic ways this work could earn or deploy. Year: 2026. Voice: terse, unsentimental, zero startup poetry.

STRICT OUTPUT RULES:
- Output a SINGLE JSON object only. No markdown fences. No prose outside JSON.

INPUT:
The user JSON has "existing_project_analysis" — ONLY that object is ground truth. Prefer: one_line_summary, what_it_actually_does, problem_it_solves, why_it_matters, core_features, positioning_label, product_intelligence. Do NOT treat tech_stack as the story.

EACH OPPORTUNITY MUST ANSWER (map into the schema fields):
- IDEA → title + what_it_is (what you ship or offer)
- WHO WOULD PAY / USE → target_customer + who_exactly_to_contact
- HOW IT WOULD BE USED → how_to_package + distribution_strategy
- WHY IT MAKES SENSE → problem + why_this_problem_is_real_now (tied to analysis, not generic market)

VALID = thin extension of this codebase, repackage for ONE niche, or apply to ONE named buyer. INVALID = greenfield platforms, "AI for everyone," 6-month builds before first dollar.

HARD RULES:
- Trace to capabilities and VALUE in the analysis — not a list of frameworks.
- ONE primary buyer per opportunity. Concrete.
- Ban: ecosystem, synergy, paradigm, transform industries, unlock value, leverage, robust, seamless.

LENGTH LIMITS:
- title: ≤12 words
- what_it_is: ≤3 lines (outcome + who uses it)
- problem: ≤2 lines (specific pain)
- why_this_problem_is_real_now: ≤2 lines
- target_customer: 1 line (role + context)
- who_exactly_to_contact: ≤3 short phrases
- how_to_package: ≤2 lines (how they'd use or buy it)
- pricing_logic: ≤2 lines
- distribution_strategy: 2–4 SHORT strings
- first_3_steps_to_validate: EXACTLY 3 concrete actions (DM, demo, call, landing page)
- risk_level: one line
- why_this_could_fail: ≤2 lines, honest

Fields per opportunity (snake_case): title, what_it_is, problem, why_this_problem_is_real_now, target_customer, who_exactly_to_contact, how_to_package, pricing_logic, distribution_strategy, first_3_steps_to_validate, risk_level, why_this_could_fail

Produce exactly 3 to 5 opportunities.

Top-level JSON: {"opportunities":[{...}, ...]}

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

    let mut parsed: OpportunityPayload =
        serde_json::from_value(v).map_err(|e| format!("Opportunity schema error: {}", e))?;

    for op in parsed.opportunities.iter_mut() {
        op.title = op.title.chars().take(90).collect();
        op.what_it_is = op.what_it_is.lines().take(3).collect::<Vec<_>>().join(" ");
        op.problem = op.problem.lines().take(2).collect::<Vec<_>>().join(" ");
        op.why_this_problem_is_real_now = op
            .why_this_problem_is_real_now
            .lines()
            .take(2)
            .collect::<Vec<_>>()
            .join(" ");
        op.pricing_logic = op.pricing_logic.lines().take(2).collect::<Vec<_>>().join(" ");
        op.why_this_could_fail = op.why_this_could_fail.lines().take(2).collect::<Vec<_>>().join(" ");
        op.distribution_strategy.truncate(3);
    }

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
