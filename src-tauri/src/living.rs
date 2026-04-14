//! Living system: ranked picks, incremental updates, evolution suggestions, positioning.
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::claude::{extract_json_text, trim_json_string_values};
use crate::db::UserProfile;
use crate::scanner::ScanResult;

/// Compact analysis for ranking (token-efficient).
pub fn compact_project_for_ranking(id: i64, name: &str, v: &serde_json::Value) -> serde_json::Value {
    json!({
        "id": id,
        "name": name,
        "one_line_summary": v.get("one_line_summary").and_then(|x| x.as_str()).unwrap_or(""),
        "problem_it_solves": v.get("problem_it_solves").and_then(|x| x.as_str()).unwrap_or(""),
        "project_intent": v.get("project_intent").and_then(|x| x.as_str()).unwrap_or(""),
        "core_features": v.get("core_features").cloned().unwrap_or(json!([])),
        "architecture_overview": v.get("architecture_overview").and_then(|x| x.as_str()).unwrap_or(""),
        "product_intelligence": v.get("product_intelligence").cloned().unwrap_or(json!({})),
    })
}

pub fn build_incremental_scan_message(
    previous_analysis: &serde_json::Value,
    folder_name: &str,
    scan: &ScanResult,
) -> String {
    let fresh: serde_json::Value = serde_json::from_str(
        &crate::claude::build_user_message(folder_name, scan, &scan.detected_stack),
    )
    .unwrap_or_else(|_| json!({}));
    json!({
        "previous_project_analysis": previous_analysis,
        "fresh_scan": fresh,
    })
    .to_string()
}

// ─── Top projects ranking ─────────────────────────────────────

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RankedPick {
    pub project_id: i64,
    pub project_name: String,
    pub rationale: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TopProjectsPayload {
    pub picks: Vec<RankedPick>,
}

pub const RANK_PROJECTS_PROMPT: &str = r#"You are a portfolio strategist. You receive JSON with "writer_context" (optional) and "projects" — each project has id, name, and a compact analysis summary (snippets from stored analysis).

TASK: Pick the TOP projects (at most 3, fewer if fewer than 3 projects exist) that best match the user's stated goal in writer_context.app_goal (e.g. get_clients → easiest to explain, relatable, sellable; portfolio → strongest story; linkedin → sharable; archive → clearest documentation).

Score mentally on: clarity of problem/solution, strength of use case, sellability / presentability, perceived polish/completeness — all inferred from text only.

RULES:
- Only use project_ids that appear in the input.
- Return 1–3 picks, ordered best first.
- rationale: 2 short sentences max per pick, specific to that project.
- If only one project exists, return one pick.

OUTPUT: single JSON object only, no markdown fences:
{"picks":[{"project_id":number,"project_name":"string","rationale":"string"},...]}

Return only the JSON object."#;

pub fn parse_top_projects(json_str: &str, allowed_ids: &[i64]) -> Result<TopProjectsPayload, String> {
    let cleaned = extract_json_text(json_str);
    let mut v: serde_json::Value = serde_json::from_str(&cleaned)
        .map_err(|e| format!("Rank response not valid JSON: {}", e))?;
    trim_json_string_values(&mut v);

    let picks_arr = v
        .get_mut("picks")
        .and_then(|x| x.as_array_mut())
        .ok_or_else(|| "Missing picks array".to_string())?;

    let mut picks: Vec<RankedPick> = Vec::new();
    let mut seen: std::collections::HashSet<i64> = std::collections::HashSet::new();
    for item in picks_arr.iter() {
        let id = item
            .get("project_id")
            .and_then(|x| x.as_i64())
            .ok_or_else(|| "pick missing project_id".to_string())?;
        if !allowed_ids.contains(&id) {
            return Err(format!("Invalid project_id {} in picks", id));
        }
        if !seen.insert(id) {
            continue;
        }
        let project_name = item
            .get("project_name")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        let rationale = item
            .get("rationale")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        if rationale.trim().is_empty() {
            return Err("Empty rationale in pick".into());
        }
        picks.push(RankedPick {
            project_id: id,
            project_name,
            rationale,
        });
    }

    if picks.len() > 3 {
        picks.truncate(3);
    }
    if picks.is_empty() && !allowed_ids.is_empty() {
        return Err("Model returned no picks".into());
    }

    Ok(TopProjectsPayload { picks })
}

pub fn build_rank_user_message(
    projects: &[serde_json::Value],
    profile: Option<&UserProfile>,
) -> String {
    let mut o = json!({ "projects": projects });
    if let Some(p) = profile {
        let role_set = p.role.as_ref().map(|s| !s.trim().is_empty()).unwrap_or(false);
        let goal_set = p.app_goal.as_ref().map(|s| !s.trim().is_empty()).unwrap_or(false);
        if role_set || !p.what_i_build.is_empty() || goal_set {
            o.as_object_mut()
                .unwrap()
                .insert("writer_context".to_string(), serde_json::to_value(p).unwrap());
        }
    }
    o.to_string()
}

// ─── Incremental project update ───────────────────────────────

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct IncrementalUpdatePayload {
    pub version_label: String,
    pub what_changed_overview: String,
    pub new_features: Vec<String>,
    pub improvements: Vec<String>,
}

pub const INCREMENTAL_UPDATE_PROMPT: &str = r#"You compare a PREVIOUS stored project analysis with a FRESH folder scan (file index + sample file contents). Output what is NEW or IMPROVED since the mental snapshot of the old analysis — not a full re-explanation.

RULES:
- Output a SINGLE JSON object only. No markdown fences.
- Infer likely new features, refactors, or UX fixes from file paths, stack, and snippets vs the old summary. If uncertain, say so briefly in what_changed_overview.
- Do NOT invent features with no evidence in the new scan.
- new_features: concrete bullets (3–8) of additions or meaningful changes.
- improvements: 2–6 bullets (polish, performance, DX, tests) if visible; else shorter.
- version_label: short label e.g. "Update — Q2" or "v2 — automation layer" (invent only a label, not a fake version number from package.json unless visible).

Keys (snake_case): version_label, what_changed_overview, new_features (array of strings), improvements (array of strings)

Return only the JSON object."#;

#[derive(Debug, Serialize)]
pub struct IncrementalUpdateResult {
    pub evolution_id: i64,
    pub payload: IncrementalUpdatePayload,
}

pub fn parse_incremental_update(json_str: &str) -> Result<IncrementalUpdatePayload, String> {
    let cleaned = extract_json_text(json_str);
    let mut v: serde_json::Value = serde_json::from_str(&cleaned)
        .map_err(|e| format!("Incremental update not valid JSON: {}", e))?;
    for key in ["new_features", "improvements"] {
        if let Some(val) = v.get_mut(key) {
            if val.is_string() {
                let s = val.as_str().unwrap_or("").to_string();
                *val = json!([s]);
            }
        }
    }
    trim_json_string_values(&mut v);
    let p: IncrementalUpdatePayload =
        serde_json::from_value(v).map_err(|e| format!("Incremental schema: {}", e))?;
    if p.new_features.is_empty() {
        return Err("new_features must not be empty".into());
    }
    Ok(p)
}

// ─── Evolution suggestions ────────────────────────────────────

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct EvolutionSuggestion {
    pub title: String,
    pub why: String,
    pub build_notes: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct EvolutionSuggestionsPayload {
    pub suggestions: Vec<EvolutionSuggestion>,
}

pub const EVOLUTION_SUGGEST_PROMPT: &str = r#"Given ONE project's stored analysis JSON, propose exactly 2 or 3 concrete next upgrades a solo builder could ship.

RULES:
- Single JSON object only. No markdown fences.
- Each suggestion must be buildable on top of the existing codebase (no greenfield products).
- Align with gaps, risks, or product_intelligence in the analysis.
- Not generic ("add tests") unless tied to a specific gap mentioned.
- title: short; why: one sentence value; build_notes: one sentence scope.

Keys: suggestions (array of 2–3 objects with title, why, build_notes)

Return only the JSON object."#;

pub fn parse_evolution_suggestions(json_str: &str) -> Result<EvolutionSuggestionsPayload, String> {
    let cleaned = extract_json_text(json_str);
    let mut v: serde_json::Value = serde_json::from_str(&cleaned)
        .map_err(|e| format!("Evolution suggestions not valid JSON: {}", e))?;
    trim_json_string_values(&mut v);
    let parsed: EvolutionSuggestionsPayload = serde_json::from_value(v)
        .map_err(|e| format!("Evolution suggestions schema: {}", e))?;
    let n = parsed.suggestions.len();
    if n < 2 || n > 3 {
        return Err(format!("Expected 2–3 suggestions, got {}", n));
    }
    Ok(parsed)
}

// ─── Positioning clarity ───────────────────────────────────────

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct PositioningPayload {
    pub category: String,
    pub primary_audience: String,
    pub one_sentence_anchor: String,
}

pub const POSITIONING_PROMPT: &str = r#"From ONE project's stored analysis JSON, produce a positioning anchor for portfolio and outreach.

RULES:
- Single JSON object only. No markdown fences.
- category: short industry/product category (specific, not "software").
- primary_audience: one line — who cares most.
- one_sentence_anchor: how to describe the project in one clear sentence — no buzzwords, no "leverage".

Keys: category, primary_audience, one_sentence_anchor

Return only the JSON object."#;

pub fn parse_positioning(json_str: &str) -> Result<PositioningPayload, String> {
    let cleaned = extract_json_text(json_str);
    let mut v: serde_json::Value = serde_json::from_str(&cleaned)
        .map_err(|e| format!("Positioning not valid JSON: {}", e))?;
    trim_json_string_values(&mut v);
    serde_json::from_value(v).map_err(|e| format!("Positioning schema: {}", e))
}
