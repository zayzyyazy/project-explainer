use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::Path;

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
pub struct ProjectDetail {
    #[serde(flatten)]
    pub row: ProjectRow,
    pub analysis: Option<serde_json::Value>,
    pub file_index_sample: Vec<String>,
    pub raw_file_list_truncated: bool,
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
            "#,
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn list_projects(conn: &Connection) -> Result<Vec<ProjectRow>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, name, path, detected_stack, one_line_summary, last_analyzed_at, created_at FROM projects ORDER BY datetime(created_at) DESC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
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

    Ok(Some(ProjectDetail {
        row: base,
        analysis,
        file_index_sample,
        raw_file_list_truncated,
    }))
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
