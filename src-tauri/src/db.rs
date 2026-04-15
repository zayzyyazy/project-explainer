use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Lightweight row for `list_projects` only — no path, stack, or analysis.
#[derive(Debug, Serialize)]
pub struct ProjectListItem {
    pub id: i64,
    pub name: String,
    pub one_line_summary: String,
    pub last_analyzed_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ProjectRow {
    pub id: i64,
    pub name: String,
    pub path: String,
    pub detected_stack: Vec<String>,
    pub one_line_summary: String,
    pub last_analyzed_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct ProjectEvolutionEntry {
    pub id: i64,
    pub label: String,
    pub new_features: Vec<String>,
    pub summary: String,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct ProjectDetail {
    #[serde(flatten)]
    pub row: ProjectRow,
    pub analysis: Option<serde_json::Value>,
    pub file_index_sample: Vec<String>,
    pub raw_file_list_truncated: bool,
    #[serde(default)]
    pub evolutions: Vec<ProjectEvolutionEntry>,
}

pub fn open_db(db_path: &Path) -> Result<Connection, String> {
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    migrate(&conn)?;
    Ok(conn)
}

fn migrate(conn: &Connection) -> Result<(), String> {
    conn
        .execute_batch(
            r#"
            PRAGMA foreign_keys = ON;
            CREATE TABLE IF NOT EXISTS projects (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                path TEXT NOT NULL UNIQUE,
                detected_stack TEXT NOT NULL,
                one_line_summary TEXT NOT NULL DEFAULT '',
                last_analyzed_at TEXT,
                created_at TEXT NOT NULL,
                file_index_json TEXT NOT NULL DEFAULT '[]',
                file_index_truncated INTEGER NOT NULL DEFAULT 0
            );
            CREATE TABLE IF NOT EXISTS analyses (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                project_id INTEGER NOT NULL,
                raw_json TEXT NOT NULL,
                architecture_overview TEXT NOT NULL,
                how_it_works TEXT NOT NULL,
                how_to_run TEXT NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_analyses_project ON analyses(project_id);
            CREATE TABLE IF NOT EXISTS idea_projects (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                source_project_id INTEGER NOT NULL,
                source_project_name TEXT NOT NULL,
                title TEXT NOT NULL,
                what_it_is TEXT NOT NULL,
                problem TEXT NOT NULL,
                why_this_problem_is_real_now TEXT NOT NULL,
                target_customer TEXT NOT NULL,
                who_exactly_to_contact TEXT NOT NULL,
                how_to_package TEXT NOT NULL,
                pricing_logic TEXT NOT NULL,
                distribution_strategy_json TEXT NOT NULL,
                first_3_steps_to_validate_json TEXT NOT NULL,
                risk_level TEXT NOT NULL,
                why_this_could_fail TEXT NOT NULL,
                saved_at TEXT NOT NULL,
                UNIQUE(source_project_id, title),
                FOREIGN KEY(source_project_id) REFERENCES projects(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_idea_projects_source ON idea_projects(source_project_id);
            CREATE INDEX IF NOT EXISTS idx_idea_projects_saved_at ON idea_projects(saved_at);
            CREATE TABLE IF NOT EXISTS user_profile (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                role TEXT,
                what_i_build_json TEXT NOT NULL DEFAULT '[]',
                app_goal TEXT,
                updated_at TEXT NOT NULL DEFAULT ''
            );
            CREATE TABLE IF NOT EXISTS project_evolutions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                project_id INTEGER NOT NULL,
                label TEXT NOT NULL,
                new_features_json TEXT NOT NULL,
                summary TEXT NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_evolutions_project ON project_evolutions(project_id);
            CREATE TABLE IF NOT EXISTS project_ai_cache (
                project_id INTEGER NOT NULL,
                kind TEXT NOT NULL,
                payload_json TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                PRIMARY KEY (project_id, kind),
                CHECK (kind IN ('opportunities', 'case_study')),
                FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE
            );
            "#,
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Local onboarding / writer context (optional). Injected into case study generation.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct UserProfile {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(default)]
    pub what_i_build: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_goal: Option<String>,
}

pub fn get_user_profile(conn: &Connection) -> Result<Option<UserProfile>, String> {
    let row: Option<(Option<String>, String, Option<String>)> = conn
        .query_row(
            "SELECT role, what_i_build_json, app_goal FROM user_profile WHERE id = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()
        .map_err(|e| e.to_string())?;

    let Some((role, w_json, app_goal)) = row else {
        return Ok(None);
    };

    let what_i_build: Vec<String> = serde_json::from_str(&w_json).unwrap_or_default();
    Ok(Some(UserProfile {
        role,
        what_i_build,
        app_goal,
    }))
}

pub fn save_user_profile(conn: &Connection, p: &UserProfile) -> Result<(), String> {
    let now = Utc::now().to_rfc3339();
    let w_json = serde_json::to_string(&p.what_i_build).map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO user_profile (id, role, what_i_build_json, app_goal, updated_at) VALUES (1, ?1, ?2, ?3, ?4)
         ON CONFLICT(id) DO UPDATE SET role = excluded.role, what_i_build_json = excluded.what_i_build_json, app_goal = excluded.app_goal, updated_at = excluded.updated_at",
        params![p.role, w_json, p.app_goal, now],
    )
    .map_err(|e| e.to_string())?;
    clear_all_case_study_caches(conn)?;
    Ok(())
}

pub fn get_ai_cache_raw(
    conn: &Connection,
    project_id: i64,
    kind: &str,
) -> Result<Option<String>, String> {
    conn.query_row(
        "SELECT payload_json FROM project_ai_cache WHERE project_id = ?1 AND kind = ?2",
        params![project_id, kind],
        |row| row.get(0),
    )
    .optional()
    .map_err(|e| e.to_string())
}

pub fn set_ai_cache(
    conn: &Connection,
    project_id: i64,
    kind: &str,
    payload_json: &str,
) -> Result<(), String> {
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO project_ai_cache (project_id, kind, payload_json, updated_at) VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(project_id, kind) DO UPDATE SET payload_json = excluded.payload_json, updated_at = excluded.updated_at",
        params![project_id, kind, payload_json, now],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn invalidate_project_ai_caches(conn: &Connection, project_id: i64) -> Result<(), String> {
    conn.execute(
        "DELETE FROM project_ai_cache WHERE project_id = ?1",
        params![project_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn delete_ai_cache(conn: &Connection, project_id: i64, kind: &str) -> Result<(), String> {
    conn.execute(
        "DELETE FROM project_ai_cache WHERE project_id = ?1 AND kind = ?2",
        params![project_id, kind],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn clear_all_case_study_caches(conn: &Connection) -> Result<(), String> {
    conn.execute("DELETE FROM project_ai_cache WHERE kind = 'case_study'", [])
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Latest stored analysis JSON only (no evolutions) — for ranking / light reads.
pub fn get_latest_analysis_json(
    conn: &Connection,
    project_id: i64,
) -> Result<Option<serde_json::Value>, String> {
    let raw: Option<String> = conn
        .query_row(
            "SELECT raw_json FROM analyses WHERE project_id = ?1 ORDER BY datetime(created_at) DESC LIMIT 1",
            params![project_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    Ok(raw.and_then(|s| serde_json::from_str(&s).ok()))
}

pub fn list_projects(conn: &Connection) -> Result<Vec<ProjectListItem>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, name, one_line_summary, last_analyzed_at FROM projects ORDER BY datetime(created_at) DESC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok(ProjectListItem {
                id: row.get(0)?,
                name: row.get(1)?,
                one_line_summary: row.get(2)?,
                last_analyzed_at: row.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

pub fn get_project_by_id(conn: &Connection, id: i64) -> Result<Option<ProjectDetail>, String> {
    let row: Option<ProjectRow> = conn
        .query_row(
            "SELECT id, name, path, detected_stack, one_line_summary, last_analyzed_at, created_at FROM projects WHERE id = ?1",
            params![id],
            |row| {
                let stack_json: String = row.get(3)?;
                let stack: Vec<String> = serde_json::from_str(&stack_json).unwrap_or_default();
                Ok(ProjectRow {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    path: row.get(2)?,
                    detected_stack: stack,
                    one_line_summary: row.get(4)?,
                    last_analyzed_at: row.get(5)?,
                    created_at: row.get(6)?,
                })
            },
        )
        .optional()
        .map_err(|e| e.to_string())?;

    let Some(mut base) = row else {
        return Ok(None);
    };

    let analysis_row: Option<(String, String, String, String)> = conn
        .query_row(
            "SELECT raw_json, architecture_overview, how_it_works, how_to_run FROM analyses WHERE project_id = ?1 ORDER BY datetime(created_at) DESC LIMIT 1",
            params![id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .optional()
        .map_err(|e| e.to_string())?;

    let analysis = analysis_row.and_then(|(raw, _, _, _)| serde_json::from_str::<serde_json::Value>(&raw).ok());

    let (file_index_sample, raw_file_list_truncated): (Vec<String>, bool) = conn
        .query_row(
            "SELECT file_index_json, file_index_truncated FROM projects WHERE id = ?1",
            params![id],
            |row| {
                let j: String = row.get(0)?;
                let truncated: i64 = row.get(1)?;
                let list: Vec<String> = serde_json::from_str(&j).unwrap_or_default();
                Ok((list, truncated != 0))
            },
        )
        .optional()
        .map_err(|e| e.to_string())?
        .unwrap_or_else(|| (vec![], false));

    if let Some(ref v) = analysis {
        if let Some(s) = v.get("one_line_summary").and_then(|x| x.as_str()) {
            base.one_line_summary = s.to_string();
        }
    }

    let evolutions = list_project_evolutions(conn, id)?;

    Ok(Some(ProjectDetail {
        row: base,
        analysis,
        file_index_sample,
        raw_file_list_truncated,
        evolutions,
    }))
}

pub fn list_project_evolutions(
    conn: &Connection,
    project_id: i64,
) -> Result<Vec<ProjectEvolutionEntry>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, label, new_features_json, summary, created_at FROM project_evolutions WHERE project_id = ?1 ORDER BY datetime(created_at) ASC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![project_id], |row| {
            let nf: String = row.get(2)?;
            let feats: Vec<String> = serde_json::from_str(&nf).unwrap_or_default();
            Ok(ProjectEvolutionEntry {
                id: row.get(0)?,
                label: row.get(1)?,
                new_features: feats,
                summary: row.get(3)?,
                created_at: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

pub fn insert_project_evolution(
    conn: &Connection,
    project_id: i64,
    label: &str,
    new_features: &[String],
    summary: &str,
) -> Result<i64, String> {
    let now = Utc::now().to_rfc3339();
    let nf = serde_json::to_string(new_features).map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO project_evolutions (project_id, label, new_features_json, summary, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![project_id, label, nf, summary, now],
    )
    .map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

pub fn path_exists(conn: &Connection, path: &str) -> Result<bool, String> {
    let n: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM projects WHERE path = ?1",
            params![path],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    Ok(n > 0)
}

#[derive(Deserialize)]
pub struct InsertProject {
    pub name: String,
    pub path: String,
    pub detected_stack: Vec<String>,
    pub file_index_json: Vec<String>,
    pub file_index_truncated: bool,
}

pub fn insert_project(conn: &Connection, p: InsertProject) -> Result<i64, String> {
    let now = Utc::now().to_rfc3339();
    let stack = serde_json::to_string(&p.detected_stack).map_err(|e| e.to_string())?;
    let idx = serde_json::to_string(&p.file_index_json).map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO projects (name, path, detected_stack, one_line_summary, last_analyzed_at, created_at, file_index_json, file_index_truncated) VALUES (?1, ?2, ?3, '', NULL, ?4, ?5, ?6)",
        params![
            p.name,
            p.path,
            stack,
            now,
            idx,
            if p.file_index_truncated { 1 } else { 0 }
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

pub fn update_project_after_analysis(
    conn: &Connection,
    project_id: i64,
    one_line_summary: &str,
    detected_stack: &[String],
    raw_analysis: &serde_json::Value,
    architecture: &str,
    how_works: &str,
    how_run: &str,
) -> Result<(), String> {
    let now = Utc::now().to_rfc3339();
    let raw = serde_json::to_string(raw_analysis).map_err(|e| e.to_string())?;
    let stack = serde_json::to_string(detected_stack).map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE projects SET one_line_summary = ?1, last_analyzed_at = ?2, detected_stack = ?3 WHERE id = ?4",
        params![one_line_summary, now, stack, project_id],
    )
    .map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO analyses (project_id, raw_json, architecture_overview, how_it_works, how_to_run, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![project_id, raw, architecture, how_works, how_run, now],
    )
    .map_err(|e| e.to_string())?;
    invalidate_project_ai_caches(conn, project_id)?;
    Ok(())
}

pub fn delete_project(conn: &Connection, id: i64) -> Result<(), String> {
    conn
        .execute("DELETE FROM projects WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn get_project_path(conn: &Connection, id: i64) -> Result<Option<String>, String> {
    conn.query_row("SELECT path FROM projects WHERE id = ?1", params![id], |row| {
        row.get(0)
    })
    .optional()
    .map_err(|e| e.to_string())
}

//
// ───────────────────────────────────────────────────────────
// Idea projects (saved opportunities) — separate from analyses
// ───────────────────────────────────────────────────────────
//

#[derive(Debug, Deserialize)]
pub struct SaveIdeaProjectInput {
    pub source_project_id: i64,
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

#[derive(Debug, Serialize)]
pub struct IdeaProject {
    pub id: i64,
    pub source_project_id: i64,
    pub source_project_name: String,
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
    pub saved_at: String,
}

fn row_to_idea_project(row: &rusqlite::Row<'_>) -> rusqlite::Result<IdeaProject> {
    let dist_json: String = row.get(11)?;
    let steps_json: String = row.get(12)?;
    let dist: Vec<String> = serde_json::from_str(&dist_json).unwrap_or_default();
    let steps: Vec<String> = serde_json::from_str(&steps_json).unwrap_or_default();
    Ok(IdeaProject {
        id: row.get(0)?,
        source_project_id: row.get(1)?,
        source_project_name: row.get(2)?,
        title: row.get(3)?,
        what_it_is: row.get(4)?,
        problem: row.get(5)?,
        why_this_problem_is_real_now: row.get(6)?,
        target_customer: row.get(7)?,
        who_exactly_to_contact: row.get(8)?,
        how_to_package: row.get(9)?,
        pricing_logic: row.get(10)?,
        distribution_strategy: dist,
        first_3_steps_to_validate: steps,
        risk_level: row.get(13)?,
        why_this_could_fail: row.get(14)?,
        saved_at: row.get(15)?,
    })
}

pub fn save_idea_project(conn: &Connection, input: SaveIdeaProjectInput) -> Result<i64, String> {
    let source_name: String = conn
        .query_row(
            "SELECT name FROM projects WHERE id = ?1",
            params![input.source_project_id],
            |row| row.get(0),
        )
        .map_err(|_| "Source project not found.".to_string())?;

    let exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM idea_projects WHERE source_project_id = ?1 AND title = ?2",
            params![input.source_project_id, input.title],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    if exists > 0 {
        return Err(
            "This idea is already saved for this project (same title).".to_string(),
        );
    }

    let now = Utc::now().to_rfc3339();
    let dist = serde_json::to_string(&input.distribution_strategy).map_err(|e| e.to_string())?;
    let steps =
        serde_json::to_string(&input.first_3_steps_to_validate).map_err(|e| e.to_string())?;

    conn.execute(
        r#"INSERT INTO idea_projects (
            source_project_id, source_project_name, title, what_it_is, problem,
            why_this_problem_is_real_now, target_customer, who_exactly_to_contact,
            how_to_package, pricing_logic, distribution_strategy_json,
            first_3_steps_to_validate_json, risk_level, why_this_could_fail, saved_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)"#,
        params![
            input.source_project_id,
            source_name,
            input.title,
            input.what_it_is,
            input.problem,
            input.why_this_problem_is_real_now,
            input.target_customer,
            input.who_exactly_to_contact,
            input.how_to_package,
            input.pricing_logic,
            dist,
            steps,
            input.risk_level,
            input.why_this_could_fail,
            now,
        ],
    )
    .map_err(|e| e.to_string())?;

    Ok(conn.last_insert_rowid())
}

pub fn list_idea_projects(conn: &Connection) -> Result<Vec<IdeaProject>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, source_project_id, source_project_name, title, what_it_is, problem, \
             why_this_problem_is_real_now, target_customer, who_exactly_to_contact, \
             how_to_package, pricing_logic, distribution_strategy_json, \
             first_3_steps_to_validate_json, risk_level, why_this_could_fail, saved_at \
             FROM idea_projects ORDER BY datetime(saved_at) DESC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| row_to_idea_project(row))
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

pub fn get_idea_project(conn: &Connection, id: i64) -> Result<Option<IdeaProject>, String> {
    let row = conn
        .query_row(
            "SELECT id, source_project_id, source_project_name, title, what_it_is, problem, \
             why_this_problem_is_real_now, target_customer, who_exactly_to_contact, \
             how_to_package, pricing_logic, distribution_strategy_json, \
             first_3_steps_to_validate_json, risk_level, why_this_could_fail, saved_at \
             FROM idea_projects WHERE id = ?1",
            params![id],
            |row| row_to_idea_project(row),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    Ok(row)
}

pub fn delete_idea_project(conn: &Connection, id: i64) -> Result<(), String> {
    let n = conn
        .execute("DELETE FROM idea_projects WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    if n == 0 {
        return Err("Saved idea not found.".to_string());
    }
    Ok(())
}
