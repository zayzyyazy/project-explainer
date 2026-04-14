use std::path::Path;
use std::sync::Mutex;

use tauri::{Manager, State};

mod case_study;
mod claude;
mod db;
mod living;
mod openai;
mod opportunities;
mod scanner;

use db::{
    InsertProject, IdeaProject, ProjectDetail, ProjectRow, SaveIdeaProjectInput, UserProfile,
};
use case_study::CaseStudyPayload;
use living::{
    EvolutionSuggestionsPayload, IncrementalUpdateResult, PositioningPayload, TopProjectsPayload,
};
use opportunities::OpportunityPayload;

struct AppState {
    db: Mutex<rusqlite::Connection>,
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

//
// ───────────────────────────────────────────────────────────
// BASIC COMMANDS
// ───────────────────────────────────────────────────────────
//

#[tauri::command]
fn list_projects(state: State<'_, AppState>) -> Result<Vec<ProjectRow>, String> {
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
    id: i64,
) -> Result<OpportunityPayload, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;

    let detail = db::get_project_by_id(&conn, id)?
        .ok_or_else(|| "Project not found".to_string())?;

    let analysis = detail.analysis.ok_or_else(|| {
        "No stored analysis for this project. Complete analysis first, then try again.".to_string()
    })?;

    drop(conn);

    run_generate_opportunities_from_analysis(&analysis)
}

#[tauri::command]
fn generate_case_study(state: State<'_, AppState>, id: i64) -> Result<CaseStudyPayload, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;

    let detail = db::get_project_by_id(&conn, id)?
        .ok_or_else(|| "Project not found".to_string())?;

    let analysis = detail.analysis.ok_or_else(|| {
        "No stored analysis for this project. Complete analysis first, then try again.".to_string()
    })?;

    let profile = db::get_user_profile(&conn)?;

    drop(conn);

    run_generate_case_study_from_analysis(&analysis, profile.as_ref())
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
        ])
        .run(tauri::generate_context!())
        .expect("failed to run app");
}