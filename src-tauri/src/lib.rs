
use std::path::Path;
use std::sync::Mutex;

use tauri::{Manager, State};

mod claude;
mod db;
mod scanner;

use db::{InsertProject, ProjectDetail, ProjectRow};

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

fn anthropic_key() -> Result<String, String> {
    let key = std::env::var("ANTHROPIC_API_KEY")
        .map_err(|_| "Missing ANTHROPIC_API_KEY".to_string())?;

    let key = key.trim().to_string();

    if !key.starts_with("sk-ant-") {
        return Err("Invalid Anthropic key format".into());
    }

    Ok(key)
}

fn anthropic_model() -> String {
    match std::env::var("ANTHROPIC_MODEL") {
        Ok(s) => {
            let t = s.trim();
            if t.is_empty() {
                "claude-3-5-sonnet-20241022".to_string()
            } else {
                t.to_string()
            }
        }
        Err(_) => "claude-3-5-sonnet-20241022".to_string(),
    }
}

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
fn export_markdown(
    state: State<'_, AppState>,
    id: i64,
    file_path: String,
) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let detail = db::get_project_by_id(&conn, id)?
        .ok_or_else(|| "Project not found".to_string())?;

    let analysis = detail
        .analysis
        .ok_or_else(|| "No analysis to export".to_string())?;

    let markdown = markdown_export(&analysis)?;
    std::fs::write(&file_path, markdown).map_err(|e| e.to_string())?;
    Ok(())
}

fn markdown_export(v: &serde_json::Value) -> Result<String, String> {
    let name = v
        .get("project_name")
        .and_then(|x| x.as_str())
        .unwrap_or("Project");

    let mut out = String::new();
    out.push_str(&format!("# {}\n\n", name));

    if let Some(s) = v.get("one_line_summary").and_then(|x| x.as_str()) {
        out.push_str(&format!("{}\n\n", s));
    }

    let text_sections = [
        ("Intent", "project_intent"),
        ("When built", "when_built"),
        ("Deep explanation", "deep_explanation"),
        ("Full narrative explanation", "full_narrative_explanation"),
        ("Problem it solves", "problem_it_solves"),
        ("Why it matters", "why_it_matters"),
        ("Architecture overview", "architecture_overview"),
        ("How to run", "how_to_run"),
    ];

    for (title, key) in text_sections {
        if let Some(s) = v.get(key).and_then(|x| x.as_str()) {
            out.push_str(&format!("## {}\n\n{}\n\n", title, s));
        }
    }

    let list_sections = [
        ("Core features", "core_features"),
        ("Key flows", "key_flows"),
        ("Tech stack", "tech_stack"),
        ("How it works step by step", "how_it_works_step_by_step"),
        ("Design decisions", "design_decisions"),
        ("Tradeoffs and limitations", "tradeoffs_and_limitations"),
        ("Example outputs", "example_outputs"),
    ];

    for (title, key) in list_sections {
        if let Some(arr) = v.get(key).and_then(|x| x.as_array()) {
            out.push_str(&format!("## {}\n\n", title));
            for item in arr {
                if let Some(s) = item.as_str() {
                    out.push_str(&format!("- {}\n", s));
                }
            }
            out.push('\n');
        }
    }

    if let Some(pi) = v.get("product_intelligence").and_then(|x| x.as_object()) {
        out.push_str("## Product Intelligence\n\n");
        if let Some(s) = pi.get("category").and_then(|x| x.as_str()) {
            out.push_str(&format!("**Category:** {}\n\n", s));
        }
        if let Some(s) = pi.get("product_stage").and_then(|x| x.as_str()) {
            out.push_str(&format!("**Product stage:** {}\n\n", s));
        }
        let pi_lists = [
            ("Target users", "target_users"),
            ("Use cases", "use_cases"),
            ("Monetization models", "monetization_models"),
            ("Distribution channels", "distribution_channels"),
            ("Strengths", "strengths"),
            ("Risks", "risks"),
            ("What's missing", "what_is_missing"),
        ];
        for (title, key) in pi_lists {
            if let Some(arr) = pi.get(key).and_then(|x| x.as_array()) {
                out.push_str(&format!("### {}\n\n", title));
                for item in arr {
                    if let Some(s) = item.as_str() {
                        out.push_str(&format!("- {}\n", s));
                    }
                }
                out.push('\n');
            }
        }
    }

    if let Some(arr) = v.get("important_files").and_then(|x| x.as_array()) {
        out.push_str("## Important files\n\n");
        for item in arr {
            let path = item.get("path").and_then(|x| x.as_str()).unwrap_or("");
            let why = item
                .get("why_it_matters")
                .and_then(|x| x.as_str())
                .unwrap_or("");
            let confidence = item
                .get("confidence_notes")
                .and_then(|x| x.as_str())
                .unwrap_or("");
            let gaps = item
                .get("possible_gaps_or_uncertainties")
                .and_then(|x| x.as_str())
                .unwrap_or("");

            out.push_str(&format!("### `{}`\n\n", path));

            if !why.is_empty() {
                out.push_str(&format!("{}\n\n", why));
            }
            if !confidence.is_empty() {
                out.push_str(&format!("**Confidence:** {}\n\n", confidence));
            }
            if !gaps.is_empty() {
                out.push_str(&format!("**Gaps:** {}\n\n", gaps));
            }
        }
    }

    Ok(out)
}

fn run_analysis_for_path(path_str: &str) -> Result<claude::AnalysisPayload, String> {
    let root = Path::new(path_str);
    let scan = scanner::scan_project(root)?;

    if scan.file_index.is_empty() {
        return Err(
            "No text files found in this folder (or everything was ignored as generated/binary)."
                .to_string(),
        );
    }

    if scan.selected_files.is_empty() {
        return Err("Could not pick files to read for analysis.".to_string());
    }

    let folder_name = root
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("project")
        .to_string();

    let user_message = claude::build_user_message(&folder_name, &scan, &scan.detected_stack);

    let api_key = anthropic_key()?;
    let model = anthropic_model();
    let raw = claude::call_claude(&api_key, &model, &user_message)?;

    claude::parse_and_validate(&raw)
}

#[tauri::command]
fn import_project(state: State<'_, AppState>, path: String) -> Result<ProjectDetail, String> {
    let root = Path::new(&path);
    let canonical = std::fs::canonicalize(root).map_err(|e| format!("Invalid folder: {}", e))?;
    let path_str = canonical.to_string_lossy().to_string();

    let conn = state.db.lock().map_err(|e| e.to_string())?;

    if db::path_exists(&conn, &path_str)? {
        return Err("This folder is already in your library.".to_string());
    }

    let scan = scanner::scan_project(&canonical)?;
    let name = canonical
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("project")
        .to_string();

    let file_index_sample = scanner::index_sample_paths(&scan.file_index, 120);

    let id = db::insert_project(
        &conn,
        InsertProject {
            name: name.clone(),
            path: path_str.clone(),
            detected_stack: scan.detected_stack.clone(),
            file_index_json: file_index_sample,
            file_index_truncated: scan.index_truncated,
        },
    )?;

    match run_analysis_for_path(&path_str) {
        Ok(analysis) => {
            let summary = analysis.one_line_summary.clone();
            let stack = analysis.tech_stack.clone();
            let architecture = analysis.architecture_overview.clone();
            let how_it_works = analysis.how_it_works_step_by_step.join("\n");
            let how_to_run = analysis.how_to_run.clone();
            let raw_json = serde_json::to_value(&analysis).map_err(|e| e.to_string())?;

            db::update_project_after_analysis(
                &conn,
                id,
                &summary,
                &stack,
                &raw_json,
                &architecture,
                &how_it_works,
                &how_to_run,
            )?;
        }
        Err(e) => {
            drop(conn);
            return Err(format!("Import saved, but analysis failed: {}", e));
        }
    }

    drop(conn);

    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::get_project_by_id(&conn, id)?
        .ok_or_else(|| "Failed to load project after import".to_string())
}

#[tauri::command]
fn reanalyze_project(state: State<'_, AppState>, id: i64) -> Result<ProjectDetail, String> {
    let path_str = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        db::get_project_path(&conn, id)?
            .ok_or_else(|| "Project not found".to_string())?
    };

    let analysis = run_analysis_for_path(&path_str)?;

    let summary = analysis.one_line_summary.clone();
    let stack = analysis.tech_stack.clone();
    let architecture = analysis.architecture_overview.clone();
    let how_it_works = analysis.how_it_works_step_by_step.join("\n");
    let how_to_run = analysis.how_to_run.clone();
    let raw_json = serde_json::to_value(&analysis).map_err(|e| e.to_string())?;

    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::update_project_after_analysis(
        &conn,
        id,
        &summary,
        &stack,
        &raw_json,
        &architecture,
        &how_it_works,
        &how_to_run,
    )?;

    db::get_project_by_id(&conn, id)?
        .ok_or_else(|| "Project disappeared".to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    dotenvy::dotenv().ok();
    
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let dir = app_db_dir()?;
            std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

            let path = db_path()?;
            let conn = db::open_db(&path)?;

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
            export_markdown,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}