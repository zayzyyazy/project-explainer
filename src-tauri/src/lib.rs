use std::path::Path;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{Manager, State};

mod case_study;
mod claude;
mod db;
mod living;
mod openai;
mod opportunities;
mod scanner;

use db::{
    InsertProject, IdeaProject, ProjectDetail, ProjectListItem, SaveIdeaProjectInput, UserProfile,
};
use case_study::CaseStudyPayload;
use living::{
    EvolutionSuggestionsPayload, IncrementalUpdateResult, PositioningPayload, TopProjectsPayload,
};
use opportunities::OpportunityPayload;

#[derive(Serialize)]
pub struct AiOpportunitiesResult {
    pub payload: OpportunityPayload,
    pub from_cache: bool,
}

#[derive(Serialize)]
pub struct AiCaseStudyResult {
    pub payload: CaseStudyPayload,
    pub from_cache: bool,
}

/// IPC payload for cache-first AI commands (`regenerate` defaults to false if omitted).
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GenerateAiArgs {
    id: i64,
    #[serde(default)]
    regenerate: bool,
}

struct AppState {
    db: Mutex<rusqlite::Connection>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeStatus {
    has_api_key: bool,
    has_profile: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct ProjectImportancePayload {
    top_insights: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LinkedinArgs {
    id: i64,
    length: String,
    focus: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LinkedinResult {
    text: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExportProjectArgs {
    id: i64,
    output_dir: String,
    include_opportunities: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportBundleResult {
    written_files: Vec<String>,
}

fn app_db_dir() -> Result<std::path::PathBuf, String> {
    let mut dir =
        dirs::data_local_dir().ok_or_else(|| "Could not resolve local data directory".to_string())?;
    dir.push("ProjectExplainerOS");
    Ok(dir)
}

fn db_path() -> Result<std::path::PathBuf, String> {
    Ok(app_db_dir()?.join("project-explainer.db"))
}

//
// ───────────────────────────────────────────────────────────
// ENV + CONFIG
// ───────────────────────────────────────────────────────────
//

fn anthropic_key() -> Result<String, String> {
    let key = std::env::var("ANTHROPIC_API_KEY")
        .map_err(|_| "ANTHROPIC_API_KEY is not set. Add it to src-tauri/.env".to_string())?;

    let key = key.trim().to_string();

    if !key.starts_with("sk-ant-") {
        return Err("Invalid Anthropic key format".into());
    }

    Ok(key)
}

fn anthropic_model() -> String {
    std::env::var("ANTHROPIC_MODEL")
        .unwrap_or_else(|_| "claude-sonnet-4-6".to_string())
        .trim()
        .to_string()
}

fn ai_provider() -> String {
    std::env::var("AI_PROVIDER")
        .unwrap_or_else(|_| "anthropic".to_string())
        .trim()
        .to_lowercase()
}

fn openai_key() -> Result<String, String> {
    let key = std::env::var("OPENAI_API_KEY").map_err(|_| {
        "OPENAI_API_KEY is not set. Add it to src-tauri/.env when AI_PROVIDER=openai.".to_string()
    })?;

    Ok(key.trim().to_string())
}

fn openai_model() -> String {
    std::env::var("OPENAI_MODEL")
        .unwrap_or_else(|_| "gpt-4o-mini".to_string())
        .trim()
        .to_string()
}

pub(crate) fn complete_ai_with_system(
    system_prompt: &str,
    user_message: &str,
) -> Result<String, String> {
    match ai_provider().as_str() {
        "openai" => {
            let api_key = openai_key()?;
            let model = openai_model();
            openai::call_openai_with_system(&api_key, &model, system_prompt, user_message)
        }
        _ => {
            let api_key = anthropic_key()?;
            let model = anthropic_model();
            claude::call_claude_with_system(&api_key, &model, system_prompt, user_message)
        }
    }
}

fn truncate_chars(s: &str, max: usize) -> String {
    s.chars().take(max).collect::<String>().trim().to_string()
}

fn clean_line(s: &str) -> String {
    let banned = ["robust", "leveraged", "seamless", "powerful"];
    let mut out = s.trim().to_string();
    for b in banned {
        out = out.replace(b, "");
        out = out.replace(&b.to_uppercase(), "");
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn top_insights_from_analysis(a: &serde_json::Value) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(s) = a.get("one_line_summary").and_then(|x| x.as_str()) {
        out.push(format!("Summary: {}", truncate_chars(&clean_line(s), 150)));
    }
    if let Some(f) = a
        .get("core_features")
        .and_then(|x| x.as_array())
        .and_then(|arr| arr.first())
        .and_then(|x| x.as_str())
    {
        out.push(format!("Feature: {}", truncate_chars(&clean_line(f), 150)));
    }
    if let Some(u) = a.get("problem_it_solves").and_then(|x| x.as_str()) {
        out.push(format!("Use case: {}", truncate_chars(&clean_line(u), 150)));
    }
    if out.is_empty() {
        out.push("Summary: Analysis available.".to_string());
    }
    out.truncate(3);
    out
}

fn build_linkedin_text(a: &serde_json::Value, length: &str, focus: &str) -> String {
    let name = a
        .get("project_name")
        .and_then(|x| x.as_str())
        .unwrap_or("my project");
    let summary = clean_line(
        a.get("one_line_summary")
            .and_then(|x| x.as_str())
            .unwrap_or("Built and shipped a practical tool."),
    );
    let stack = a
        .get("tech_stack")
        .and_then(|x| x.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .take(4)
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_else(|| "Tauri + React".to_string());
    let problem = clean_line(
        a.get("problem_it_solves")
            .and_then(|x| x.as_str())
            .unwrap_or("It removes repeated manual explanation work."),
    );
    let outcome = clean_line(
        a.get("why_it_matters")
            .and_then(|x| x.as_str())
            .unwrap_or("It speeds up delivery and communication."),
    );

    let short = match focus {
        "describe_stack" => format!("Shipped {} with {}.\n{}\nBuilt for real usage.", name, stack, summary),
        "describe_problem" => format!("Built {} to solve this: {}.\n{}\nNow the workflow is clearer and faster.", name, truncate_chars(&problem, 110), summary),
        "describe_outcome" => format!("{} is now shipping faster.\n{}\n{}", name, summary, truncate_chars(&outcome, 100)),
        _ => format!("Built {}.\n{}\n{}", name, summary, truncate_chars(&outcome, 100)),
    };
    if length == "short" {
        return short;
    }
    match focus {
        "describe_stack" => format!(
            "Built {} using {}.\n{}\nFocused on simple flows and local-first behavior.\nNo extra backend layer.\nOutcome: {}",
            name,
            stack,
            summary,
            truncate_chars(&outcome, 120)
        ),
        "describe_problem" => format!(
            "Built {} to solve a concrete issue.\nProblem: {}\nApproach: local scan + structured analysis + reusable outputs.\n{}\nOutcome: {}",
            name,
            truncate_chars(&problem, 120),
            summary,
            truncate_chars(&outcome, 120)
        ),
        "describe_outcome" => format!(
            "{} now delivers this result:\n{}\n{}\nStack: {}\nShipped and used in real workflows.",
            name,
            truncate_chars(&outcome, 120),
            summary,
            stack
        ),
        _ => format!(
            "Built {} and shipped it end-to-end.\n{}\nProblem: {}\nStack: {}\nOutcome: {}",
            name,
            summary,
            truncate_chars(&problem, 100),
            stack,
            truncate_chars(&outcome, 110)
        ),
    }
}

//
// ───────────────────────────────────────────────────────────
// BASIC COMMANDS
// ───────────────────────────────────────────────────────────
//

#[tauri::command]
fn list_projects(state: State<'_, AppState>) -> Result<Vec<ProjectListItem>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::list_projects(&conn)
}

#[tauri::command]
fn get_project(state: State<'_, AppState>, id: i64) -> Result<Option<ProjectDetail>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::get_project_by_id(&conn, id)
}

#[tauri::command]
fn delete_project(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::delete_project(&conn, id)
}

#[tauri::command]
fn save_idea_project(
    state: State<'_, AppState>,
    input: SaveIdeaProjectInput,
) -> Result<i64, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::save_idea_project(&conn, input)
}

#[tauri::command]
fn list_idea_projects(state: State<'_, AppState>) -> Result<Vec<IdeaProject>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::list_idea_projects(&conn)
}

#[tauri::command]
fn get_idea_project(state: State<'_, AppState>, id: i64) -> Result<Option<IdeaProject>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::get_idea_project(&conn, id)
}

#[tauri::command]
fn delete_idea_project(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::delete_idea_project(&conn, id)
}

#[tauri::command]
fn get_user_profile(state: State<'_, AppState>) -> Result<Option<UserProfile>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::get_user_profile(&conn)
}

#[tauri::command]
fn save_user_profile(state: State<'_, AppState>, profile: UserProfile) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::save_user_profile(&conn, &profile)
}

//
// ───────────────────────────────────────────────────────────
// ANALYSIS CORE
// ───────────────────────────────────────────────────────────
//

fn run_analysis_for_path(path_str: &str) -> Result<claude::AnalysisPayload, String> {
    let root = Path::new(path_str);
    let scan = scanner::scan_project(root)?;

    if scan.file_index.is_empty() {
        return Err("No readable files found.".into());
    }

    if scan.selected_files.is_empty() {
        return Err("No files selected for analysis.".into());
    }

    let folder_name = root
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("project")
        .to_string();

    let user_message =
        claude::build_user_message(&folder_name, &scan, &scan.detected_stack);

    let raw = match ai_provider().as_str() {
        "openai" => {
            let api_key = openai_key()?;
            let model = openai_model();
            openai::call_openai(&api_key, &model, &user_message)?
        }
        _ => {
            let api_key = anthropic_key()?;
            let model = anthropic_model();
            claude::call_claude(&api_key, &model, &user_message)?
        }
    };

    claude::parse_and_validate(&raw)
}

fn run_generate_opportunities_from_analysis(
    analysis: &serde_json::Value,
) -> Result<OpportunityPayload, String> {
    let user_message = opportunities::build_opportunity_user_message(analysis);

    let raw = match ai_provider().as_str() {
        "openai" => {
            let api_key = openai_key()?;
            let model = openai_model();
            openai::call_openai_with_system(
                &api_key,
                &model,
                opportunities::OPPORTUNITY_SYSTEM_PROMPT,
                &user_message,
            )?
        }
        _ => {
            let api_key = anthropic_key()?;
            let model = anthropic_model();
            claude::call_claude_with_system(
                &api_key,
                &model,
                opportunities::OPPORTUNITY_SYSTEM_PROMPT,
                &user_message,
            )?
        }
    };

    opportunities::parse_and_validate_opportunities(&raw)
}

fn run_generate_case_study_from_analysis(
    analysis: &serde_json::Value,
    writer_context: Option<&UserProfile>,
) -> Result<CaseStudyPayload, String> {
    let user_message = case_study::build_case_study_user_message(analysis, writer_context);

    let raw = match ai_provider().as_str() {
        "openai" => {
            let api_key = openai_key()?;
            let model = openai_model();
            openai::call_openai_with_system(
                &api_key,
                &model,
                case_study::CASE_STUDY_SYSTEM_PROMPT,
                &user_message,
            )?
        }
        _ => {
            let api_key = anthropic_key()?;
            let model = anthropic_model();
            claude::call_claude_with_system(
                &api_key,
                &model,
                case_study::CASE_STUDY_SYSTEM_PROMPT,
                &user_message,
            )?
        }
    };

    case_study::parse_and_validate_case_study(&raw)
}

//
// ───────────────────────────────────────────────────────────
// V2 — OPPORTUNITIES
// ───────────────────────────────────────────────────────────
//

#[tauri::command]
fn generate_opportunities(
    state: State<'_, AppState>,
    args: GenerateAiArgs,
) -> Result<AiOpportunitiesResult, String> {
    let id = args.id;
    let regenerate = args.regenerate;

    if !regenerate {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        if let Some(raw) = db::get_ai_cache_raw(&conn, id, "opportunities")? {
            match serde_json::from_str::<OpportunityPayload>(&raw) {
                Ok(payload) => {
                    drop(conn);
                    eprintln!("USING CACHE: opportunities project_id={}", id);
                    return Ok(AiOpportunitiesResult {
                        payload,
                        from_cache: true,
                    });
                }
                Err(e) => {
                    eprintln!(
                        "CACHE INVALID opportunities project_id={} — will call AI. {}",
                        id, e
                    );
                    let _ = db::delete_ai_cache(&conn, id, "opportunities");
                }
            }
        }
        drop(conn);
    }

    eprintln!("CALLING AI: opportunities project_id={}", id);

    let analysis = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        let detail = db::get_project_by_id(&conn, id)?
            .ok_or_else(|| "Project not found".to_string())?;
        detail
            .analysis
            .ok_or_else(|| {
                "No stored analysis for this project. Complete analysis first, then try again."
                    .to_string()
            })?
    };

    let payload = run_generate_opportunities_from_analysis(&analysis)?;

    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let json = serde_json::to_string(&payload).map_err(|e| e.to_string())?;
    db::set_ai_cache(&conn, id, "opportunities", &json)?;
    drop(conn);

    Ok(AiOpportunitiesResult {
        payload,
        from_cache: false,
    })
}

#[tauri::command]
fn generate_case_study(
    state: State<'_, AppState>,
    args: GenerateAiArgs,
) -> Result<AiCaseStudyResult, String> {
    let id = args.id;
    let regenerate = args.regenerate;

    if !regenerate {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        if let Some(raw) = db::get_ai_cache_raw(&conn, id, "case_study")? {
            match serde_json::from_str::<CaseStudyPayload>(&raw) {
                Ok(payload) => {
                    drop(conn);
                    eprintln!("USING CACHE: case_study project_id={}", id);
                    return Ok(AiCaseStudyResult {
                        payload,
                        from_cache: true,
                    });
                }
                Err(e) => {
                    eprintln!(
                        "CACHE INVALID case_study project_id={} — will call AI. {}",
                        id, e
                    );
                    let _ = db::delete_ai_cache(&conn, id, "case_study");
                }
            }
        }
        drop(conn);
    }

    eprintln!("CALLING AI: case_study project_id={}", id);

    let (analysis, profile) = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        let detail = db::get_project_by_id(&conn, id)?
            .ok_or_else(|| "Project not found".to_string())?;
        let analysis = detail
            .analysis
            .ok_or_else(|| {
                "No stored analysis for this project. Complete analysis first, then try again."
                    .to_string()
            })?;
        let profile = db::get_user_profile(&conn)?;
        (analysis, profile)
    };

    let payload = run_generate_case_study_from_analysis(&analysis, profile.as_ref())?;

    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let json = serde_json::to_string(&payload).map_err(|e| e.to_string())?;
    db::set_ai_cache(&conn, id, "case_study", &json)?;
    drop(conn);

    Ok(AiCaseStudyResult {
        payload,
        from_cache: false,
    })
}

//
// ───────────────────────────────────────────────────────────
// LIVING SYSTEM — rank, incremental update, insights
// ───────────────────────────────────────────────────────────
//

#[tauri::command]
fn rank_top_projects(state: State<'_, AppState>) -> Result<TopProjectsPayload, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let rows = db::list_projects(&conn)?;
    let profile = db::get_user_profile(&conn)?;

    let mut summaries = Vec::new();
    let mut allowed_ids = Vec::new();

    for r in rows {
        if r.last_analyzed_at.is_none() {
            continue;
        }
        if let Some(a) = db::get_latest_analysis_json(&conn, r.id)? {
            allowed_ids.push(r.id);
            summaries.push(living::compact_project_for_ranking(r.id, &r.name, &a));
        }
    }

    drop(conn);

    if summaries.is_empty() {
        return Err("No analyzed projects yet. Import and analyze at least one project.".into());
    }

    let user = living::build_rank_user_message(&summaries, profile.as_ref());
    let raw = complete_ai_with_system(living::RANK_PROJECTS_PROMPT, &user)?;
    living::parse_top_projects(&raw, &allowed_ids)
}

#[tauri::command]
fn incremental_project_update(
    state: State<'_, AppState>,
    id: i64,
) -> Result<IncrementalUpdateResult, String> {
    let path = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        db::get_project_path(&conn, id)?
            .ok_or_else(|| "Project not found".to_string())?
    };

    let root = Path::new(&path);
    let canonical = std::fs::canonicalize(root).map_err(|e| e.to_string())?;
    let scan = scanner::scan_project(&canonical)?;

    if scan.selected_files.is_empty() {
        return Err("No readable source files found for update scan.".into());
    }

    let folder_name = canonical
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("project")
        .to_string();

    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let analysis = db::get_latest_analysis_json(&conn, id)?
        .ok_or_else(|| "No stored analysis. Run full analyze first.".to_string())?;
    drop(conn);

    let user_msg = living::build_incremental_scan_message(&analysis, &folder_name, &scan);
    let raw = complete_ai_with_system(living::INCREMENTAL_UPDATE_PROMPT, &user_msg)?;
    let payload = living::parse_incremental_update(&raw)?;

    let summary = format!(
        "{}\n\nImprovements: {}",
        payload.what_changed_overview,
        payload.improvements.join("; ")
    );

    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let eid = db::insert_project_evolution(
        &conn,
        id,
        &payload.version_label,
        &payload.new_features,
        &summary,
    )?;
    drop(conn);

    Ok(IncrementalUpdateResult {
        evolution_id: eid,
        payload,
    })
}

#[tauri::command]
fn suggest_evolution_steps(
    state: State<'_, AppState>,
    id: i64,
) -> Result<EvolutionSuggestionsPayload, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let analysis = db::get_latest_analysis_json(&conn, id)?
        .ok_or_else(|| "No stored analysis for this project.".to_string())?;
    drop(conn);

    let user = serde_json::json!({ "project_analysis": analysis }).to_string();
    let raw = complete_ai_with_system(living::EVOLUTION_SUGGEST_PROMPT, &user)?;
    living::parse_evolution_suggestions(&raw)
}

#[tauri::command]
fn get_positioning_clarity(
    state: State<'_, AppState>,
    id: i64,
) -> Result<PositioningPayload, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let analysis = db::get_latest_analysis_json(&conn, id)?
        .ok_or_else(|| "No stored analysis for this project.".to_string())?;
    drop(conn);

    let user = serde_json::json!({ "project_analysis": analysis }).to_string();
    let raw = complete_ai_with_system(living::POSITIONING_PROMPT, &user)?;
    living::parse_positioning(&raw)
}

//
// ───────────────────────────────────────────────────────────
// IMPORT
// ───────────────────────────────────────────────────────────
//

#[tauri::command]
fn import_project(state: State<'_, AppState>, path: String) -> Result<ProjectDetail, String> {
    let root = Path::new(&path);
    let canonical = std::fs::canonicalize(root).map_err(|e| e.to_string())?;
    let path_str = canonical.to_string_lossy().to_string();

    let conn = state.db.lock().map_err(|e| e.to_string())?;

    if db::path_exists(&conn, &path_str)? {
        return Err("Already imported.".into());
    }

    let scan = scanner::scan_project(&canonical)?;
    let name = canonical
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("project")
        .to_string();

    let id = db::insert_project(
        &conn,
        InsertProject {
            name,
            path: path_str.clone(),
            detected_stack: scan.detected_stack.clone(),
            file_index_json: scanner::index_sample_paths(&scan.file_index, 120),
            file_index_truncated: scan.index_truncated,
        },
    )?;

    match run_analysis_for_path(&path_str) {
        Ok(analysis) => {
            let raw_json = serde_json::to_value(&analysis).map_err(|e| e.to_string())?;

            db::update_project_after_analysis(
                &conn,
                id,
                &analysis.one_line_summary,
                &analysis.tech_stack,
                &raw_json,
                &analysis.architecture_overview,
                &analysis.how_it_works_step_by_step.join("\n"),
                &analysis.how_to_run,
            )?;
        }
        Err(e) => return Err(format!("Saved but analysis failed: {}", e)),
    }

    db::get_project_by_id(&conn, id)?
        .ok_or_else(|| "Failed to load project".to_string())
}

//
// ───────────────────────────────────────────────────────────
// REANALYZE
// ───────────────────────────────────────────────────────────
//

#[tauri::command]
fn reanalyze_project(state: State<'_, AppState>, id: i64) -> Result<ProjectDetail, String> {
    let path = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        db::get_project_path(&conn, id)?
            .ok_or_else(|| "Project not found".to_string())?
    };

    let analysis = run_analysis_for_path(&path)?;
    let raw_json = serde_json::to_value(&analysis).map_err(|e| e.to_string())?;

    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::update_project_after_analysis(
        &conn,
        id,
        &analysis.one_line_summary,
        &analysis.tech_stack,
        &raw_json,
        &analysis.architecture_overview,
        &analysis.how_it_works_step_by_step.join("\n"),
        &analysis.how_to_run,
    )?;

    db::get_project_by_id(&conn, id)?
        .ok_or_else(|| "Project disappeared".to_string())
}

#[tauri::command]
fn get_runtime_status(state: State<'_, AppState>) -> Result<RuntimeStatus, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let profile = db::get_user_profile(&conn)?;
    let has_profile = profile
        .as_ref()
        .map(|p| {
            p.role.as_ref().map(|s| !s.trim().is_empty()).unwrap_or(false)
                || !p.what_i_build.is_empty()
                || p.app_goal.as_ref().map(|s| !s.trim().is_empty()).unwrap_or(false)
        })
        .unwrap_or(false);
    let has_api_key = anthropic_key().is_ok() || openai_key().is_ok();
    Ok(RuntimeStatus {
        has_api_key,
        has_profile,
    })
}

#[tauri::command]
fn get_project_importance(
    state: State<'_, AppState>,
    id: i64,
) -> Result<ProjectImportancePayload, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let analysis = db::get_latest_analysis_json(&conn, id)?
        .ok_or_else(|| "No stored analysis for this project.".to_string())?;
    Ok(ProjectImportancePayload {
        top_insights: top_insights_from_analysis(&analysis),
    })
}

#[tauri::command]
fn generate_linkedin_post(
    state: State<'_, AppState>,
    args: LinkedinArgs,
) -> Result<LinkedinResult, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let analysis = db::get_latest_analysis_json(&conn, args.id)?
        .ok_or_else(|| "No stored analysis for this project.".to_string())?;
    Ok(LinkedinResult {
        text: build_linkedin_text(&analysis, &args.length, &args.focus),
    })
}

#[tauri::command]
fn export_project_bundle(
    state: State<'_, AppState>,
    args: ExportProjectArgs,
) -> Result<ExportBundleResult, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let detail = db::get_project_by_id(&conn, args.id)?
        .ok_or_else(|| "Project not found".to_string())?;
    let analysis = detail
        .analysis
        .ok_or_else(|| "No stored analysis for this project.".to_string())?;
    let case_cache = db::get_ai_cache_raw(&conn, args.id, "case_study")?;
    let opp_cache = if args.include_opportunities {
        db::get_ai_cache_raw(&conn, args.id, "opportunities")?
    } else {
        None
    };
    drop(conn);

    let out_dir = std::path::PathBuf::from(args.output_dir);
    std::fs::create_dir_all(&out_dir).map_err(|e| e.to_string())?;
    let mut written = Vec::new();

    let summary = analysis
        .get("one_line_summary")
        .and_then(|x| x.as_str())
        .unwrap_or("No summary available");
    let problem = analysis
        .get("problem_it_solves")
        .and_then(|x| x.as_str())
        .unwrap_or("Not available");
    let outcome = analysis
        .get("why_it_matters")
        .and_then(|x| x.as_str())
        .unwrap_or("Not available");
    let case_md = if let Some(raw) = case_cache {
        if let Ok(cs) = serde_json::from_str::<CaseStudyPayload>(&raw) {
            format!(
                "# {}\n\n## Problem\n{}\n\n## Outcome\n{}\n\n## What we built\n{}\n\n## Visualization examples\n- CLI: realistic command output snippets\n- Dashboard: project summary + updates + cached assets\n- Files: generated markdown/text bundle\n",
                cs.title,
                cs.problem,
                cs.outcome,
                cs.what_we_built
                    .iter()
                    .take(4)
                    .map(|x| format!("- {}", x))
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        } else {
            format!(
                "# {}\n\n## Problem\n{}\n\n## Outcome\n{}\n",
                detail.row.name, problem, outcome
            )
        }
    } else {
        format!(
            "# {}\n\n## Problem\n{}\n\n## Outcome\n{}\n",
            detail.row.name, problem, outcome
        )
    };
    std::fs::write(out_dir.join("case-study.md"), case_md).map_err(|e| e.to_string())?;
    written.push("case-study.md".to_string());

    let pitch = format!(
        "{}\n{}\n{}",
        truncate_chars(&clean_line(summary), 120),
        truncate_chars(&clean_line(problem), 120),
        truncate_chars(&clean_line(outcome), 120)
    );
    std::fs::write(out_dir.join("short-pitch.txt"), pitch).map_err(|e| e.to_string())?;
    written.push("short-pitch.txt".to_string());

    let linkedin = build_linkedin_text(&analysis, "long", "describe_outcome");
    std::fs::write(out_dir.join("linkedin-post.txt"), linkedin).map_err(|e| e.to_string())?;
    written.push("linkedin-post.txt".to_string());

    if let Some(raw) = opp_cache {
        if let Ok(op) = serde_json::from_str::<OpportunityPayload>(&raw) {
            let mut body = String::from("# Opportunities\n\n");
            for item in op.opportunities.iter().take(5) {
                body.push_str(&format!(
                    "## {}\n{}\n\n- Problem: {}\n- Pricing: {}\n- Risk: {}\n\n",
                    item.title, item.what_it_is, item.problem, item.pricing_logic, item.risk_level
                ));
            }
            std::fs::write(out_dir.join("opportunities.md"), body).map_err(|e| e.to_string())?;
            written.push("opportunities.md".to_string());
        }
    }

    Ok(ExportBundleResult {
        written_files: written,
    })
}

//
// ───────────────────────────────────────────────────────────
// APP ENTRY
// ───────────────────────────────────────────────────────────
//

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let env_path = "/Users/zay/Desktop/Projects/project-explainer-os/src-tauri/.env";
    dotenvy::from_path(env_path).ok();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let dir = app_db_dir()?;
            std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

            let conn = db::open_db(&db_path()?)?;

            app.manage(AppState {
                db: Mutex::new(conn),
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_projects,
            get_project,
            import_project,
            delete_project,
            reanalyze_project,
            generate_opportunities,
            save_idea_project,
            list_idea_projects,
            get_idea_project,
            delete_idea_project,
            generate_case_study,
            get_user_profile,
            save_user_profile,
            rank_top_projects,
            incremental_project_update,
            suggest_evolution_steps,
            get_positioning_clarity,
            get_runtime_status,
            get_project_importance,
            generate_linkedin_post,
            export_project_bundle,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run app");
}