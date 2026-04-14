use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::claude::{extract_json_text, trim_json_string_values};
use crate::db::UserProfile;

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct CaseStudyProofBlock {
    /// e.g. cli_terminal | file_output | ui_what_you_see | other
    pub kind: String,
    pub title: String,
    /// Example terminal transcript, sample file excerpt, or concrete UI description — grounded, not fantasy
    pub body: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct CaseStudyPayload {
    pub title: String,
    pub problem: String,
    /// Why this mattered to the user/business (stakes), in plain language
    pub why_it_mattered: String,
    /// How you approached the work (process, tradeoffs, scope)
    pub approach: String,
    pub solution: String,
    pub outcome: String,
    pub outcome_basis: String,
    pub narrative: String,
    pub linkedin_hook: String,
    pub quote_ready_one_liner: String,
    pub what_we_built: Vec<String>,
    /// 1–4 concrete “proof” snippets: inferred realistic CLI output, file/sample output, or what the UI shows
    pub proof_blocks: Vec<CaseStudyProofBlock>,
}

pub const CASE_STUDY_SYSTEM_PROMPT: &str = r#"You are "Story & Code" — you turn a stored project analysis into ONE paste-ready case study that wins trust. The reader should feel: "this was really built and run."

STRICT OUTPUT RULES:
- Output a SINGLE JSON object only. No text before or after it.
- Do NOT wrap the JSON in markdown code fences. Do NOT use ``` anywhere.

INPUTS:
- "existing_project_analysis" = ground truth for what exists in the repo.
- Optional "writer_context" = who is writing and what they want (freelancer vs indie vs developer, goals). When present, shift tone:
  - freelancer → business value, client clarity, outcomes that matter to buyers; less feature laundry list.
  - indie_hacker → product narrative, user-facing value, iteration; still honest.
  - developer → stronger technical credibility, architecture wins, still tie to why it mattered.
  If writer_context is missing or empty, use a balanced freelancer-leaning tone.

GROUNDING:
- Never invent client names, revenue, metrics, or screenshots. No fake quotes.
- proof_blocks must be INFERRED but REALISTIC: plausible example CLI session, plausible sample file content snippet, or concrete UI description based on stack, example_outputs, how_to_run, key_flows, and file names in the analysis—not generic placeholders like "Lorem ipsum."
- For CLI tools: include a short fictional-but-realistic terminal transcript (command + a few lines of output) that matches what the tool likely prints.
- For file generators / exporters: show a small sample of what an output file or JSON might look like (invented content OK if clearly labeled by context in outcome_basis or block title as "illustrative example").
- For UI apps: describe 2–4 concrete screens/flows ("User sees X on first load…") grounded in architecture / features—no claiming pixel-perfect truth.

CASE STUDY STRUCTURE (business language, proposal-ready):
- problem: the situation before (short).
- why_it_mattered: stakes—time, money, risk, confusion—without melodrama.
- approach: how you tackled it (scope, sequence, key decisions)—not a tech dump.
- solution: what shipped (capabilities tied to analysis).
- outcome: results or enabled behaviors; if no metrics, qualitative and honest; outcome_basis explains inference.
- narrative: single flowing story 120–240 words, no markdown inside the string.
- proof_blocks: 1–4 items. Each has kind, title, body. body can include newlines for fake terminal output.

LENGTH:
- title: punchy, < 90 chars if possible.
- linkedin_hook: exactly 2 short sentences.
- quote_ready_one_liner: one speakable sentence.

Required JSON keys (exact snake_case):
title, problem, why_it_mattered, approach, solution, outcome, outcome_basis, narrative, linkedin_hook, quote_ready_one_liner, what_we_built (array of strings), proof_blocks (array of objects with kind, title, body)

Return only the JSON object."#;

pub fn build_case_study_user_message(
    analysis: &serde_json::Value,
    writer_context: Option<&UserProfile>,
) -> String {
    let mut payload = json!({
        "task": "generate_client_case_study",
        "existing_project_analysis": analysis,
    });

    if let Some(ctx) = writer_context {
        let role_set = ctx
            .role
            .as_ref()
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);
        let goal_set = ctx
            .app_goal
            .as_ref()
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);
        if role_set || !ctx.what_i_build.is_empty() || goal_set {
            payload
                .as_object_mut()
                .unwrap()
                .insert("writer_context".to_string(), serde_json::to_value(ctx).unwrap());
        }
    }

    payload.to_string()
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

fn normalize_proof_blocks(v: &mut serde_json::Value) {
    let Some(obj) = v.as_object_mut() else {
        return;
    };
    let Some(pb) = obj.get_mut("proof_blocks") else {
        return;
    };
    if pb.is_string() {
        *pb = json!([]);
        return;
    }
    if let Some(arr) = pb.as_array_mut() {
        for item in arr.iter_mut() {
            if let Some(o) = item.as_object_mut() {
                for key in ["title", "body", "kind"] {
                    if let Some(val) = o.get_mut(key) {
                        if val.is_null() {
                            *val = json!("");
                        }
                    }
                }
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
    normalize_proof_blocks(&mut v);
    trim_json_string_values(&mut v);

    let parsed: CaseStudyPayload =
        serde_json::from_value(v).map_err(|e| format!("Case study schema error: {}", e))?;

    if parsed.title.trim().is_empty() {
        return Err("Case study title must be non-empty".into());
    }
    if parsed.problem.trim().is_empty()
        || parsed.why_it_mattered.trim().is_empty()
        || parsed.approach.trim().is_empty()
        || parsed.solution.trim().is_empty()
    {
        return Err("problem, why_it_mattered, approach, and solution must be non-empty".into());
    }
    if parsed.outcome.trim().is_empty() || parsed.narrative.trim().is_empty() {
        return Err("outcome and narrative must be non-empty".into());
    }
    if parsed.what_we_built.is_empty() {
        return Err("what_we_built must not be empty".into());
    }

    let n = parsed.proof_blocks.len();
    if n < 1 || n > 4 {
        return Err(format!(
            "proof_blocks must have 1–4 items, got {}",
            n
        ));
    }

    for (i, b) in parsed.proof_blocks.iter().enumerate() {
        if b.kind.trim().is_empty() || b.title.trim().is_empty() || b.body.trim().is_empty() {
            return Err(format!(
                "proof_blocks[{}]: kind, title, and body must be non-empty",
                i
            ));
        }
    }

    Ok(parsed)
}
