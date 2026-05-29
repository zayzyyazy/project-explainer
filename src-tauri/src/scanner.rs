use regex::Regex;
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

/// Per-file cap for text sent to the model (README-first keeps payloads small).
const MAX_SINGLE_FILE_CHARS: usize = 14_000;
/// Hard cap on total characters across all selected files.
const MAX_TOTAL_AI_CHARS: usize = 46_000;
/// Maximum files whose contents are read and sent to AI.
const MAX_FILES_READ_CONTENT: usize = 20;
/// Shallow tree index: max depth from project root (0 = root only; 2 = root + 2 levels).
const INDEX_MAX_DEPTH: usize = 2;
/// Cap indexed path entries (names only, cheap).
const MAX_INDEX_PATHS: usize = 80;

#[derive(Debug, Clone)]
pub struct FileIndexEntry {
    pub rel_path: String,
    pub extension: String,
    pub size: u64,
}

#[derive(Debug, Clone)]
pub struct SelectedFile {
    pub rel_path: String,
    pub content: String,
}

#[derive(Debug)]
pub struct ScanResult {
    pub detected_stack: Vec<String>,
    pub file_index: Vec<FileIndexEntry>,
    pub index_truncated: bool,
    pub selected_files: Vec<SelectedFile>,
    /// True if a README was found at shallow depth and included in selection.
    pub readme_present: bool,
    /// Human-readable summary of what was scanned (for UI / debugging).
    pub scan_notes: String,
}

static IGNORE_DIRS: &[&str] = &[
    "node_modules",
    ".git",
    "dist",
    "build",
    ".next",
    "coverage",
    "venv",
    ".venv",
    "__pycache__",
    "target",
    ".turbo",
    ".nuxt",
    ".output",
    "Pods",
    ".cache",
    "vendor",
    ".svn",
    ".hg",
];

fn ignored_dir(name: &str) -> bool {
    IGNORE_DIRS.iter().any(|d| name.eq_ignore_ascii_case(d))
}

fn binary_or_skip_ext(ext: &str) -> bool {
    matches!(
        ext,
        "png" | "jpg"
            | "jpeg"
            | "gif"
            | "webp"
            | "ico"
            | "pdf"
            | "zip"
            | "gz"
            | "tar"
            | "7z"
            | "rar"
            | "wasm"
            | "exe"
            | "dll"
            | "so"
            | "dylib"
            | "bin"
            | "mp4"
            | "mov"
            | "mp3"
            | "wav"
            | "ttf"
            | "woff"
            | "woff2"
            | "eot"
            | "sqlite"
            | "db"
    )
}

fn is_lock_file(name: &str) -> bool {
    matches!(
        name,
        "package-lock.json"
            | "yarn.lock"
            | "pnpm-lock.yaml"
            | "Cargo.lock"
            | "poetry.lock"
            | "Gemfile.lock"
    ) || name.ends_with(".lock")
}

fn is_minified_path(rel: &str) -> bool {
    let lower = rel.to_ascii_lowercase();
    lower.contains(".min.") || lower.ends_with(".map")
}

fn read_text_limited(root: &Path, rel: &str, max_chars: usize) -> Option<String> {
    let full = root.join(rel);
    let bytes = fs::read(&full).ok()?;
    if bytes.len() > 1_200_000 {
        return None;
    }
    let mut s = String::from_utf8_lossy(&bytes).into_owned();
    if s.lines().take(40).any(|l| l.len() > 8000) {
        return None;
    }
    if s.len() > max_chars {
        s.truncate(max_chars);
        s.push_str("\n\n[TRUNCATED_BY_APP]");
    }
    Some(s)
}

fn extension_of(path: &str) -> String {
    Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
}

/// Shallow index only — skips ignored directories entirely (no deep crawl).
fn build_shallow_index(root_canon: &Path) -> Result<(Vec<FileIndexEntry>, bool), String> {
    let mut index: Vec<FileIndexEntry> = Vec::new();

    let walker = WalkDir::new(root_canon)
        .follow_links(false)
        .max_depth(INDEX_MAX_DEPTH)
        .into_iter()
        .filter_entry(|e| {
            if e.depth() == 0 {
                return true;
            }
            let name = e.file_name().to_str().unwrap_or("");
            !ignored_dir(name)
        });

    for entry in walker.filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let rel = path
            .strip_prefix(root_canon)
            .map_err(|e| e.to_string())?
            .to_string_lossy()
            .replace('\\', "/");

        if rel.chars().any(|c| c == '\0') {
            continue;
        }

        if rel.split('/').any(|seg| ignored_dir(seg)) {
            continue;
        }

        let ext = extension_of(&rel);
        if binary_or_skip_ext(&ext) {
            continue;
        }

        let meta = entry.metadata().map_err(|e| e.to_string())?;
        index.push(FileIndexEntry {
            rel_path: rel,
            extension: ext,
            size: meta.len(),
        });

        if index.len() >= MAX_INDEX_PATHS {
            break;
        }
    }

    index.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
    let truncated = index.len() >= MAX_INDEX_PATHS;
    Ok((index, truncated))
}

fn detect_stack(root: &Path, index: &[FileIndexEntry]) -> Vec<String> {
    let mut signals: HashSet<String> = HashSet::new();
    let names: HashSet<String> = index.iter().map(|e| e.rel_path.clone()).collect();

    for n in &names {
        let base = Path::new(n).file_name().and_then(|s| s.to_str()).unwrap_or("");
        match base {
            "package.json" => {
                signals.insert("Node / npm".into());
            }
            "requirements.txt" | "Pipfile" => {
                signals.insert("Python".into());
            }
            "pyproject.toml" => {
                signals.insert("Python".into());
            }
            "Cargo.toml" => {
                signals.insert("Rust".into());
            }
            "tauri.conf.json" => {
                signals.insert("Tauri".into());
            }
            "tsconfig.json" => {
                signals.insert("TypeScript".into());
            }
            _ => {}
        }
        if n.ends_with("prisma/schema.prisma") {
            signals.insert("Prisma".into());
        }
        if Regex::new(r"(?i)next\.config\.(js|mjs|cjs|ts)$")
            .ok()
            .map(|re| re.is_match(n))
            .unwrap_or(false)
        {
            signals.insert("Next.js".into());
        }
        if Regex::new(r"(?i)vite\.config\.(ts|js|mts|cts)$")
            .ok()
            .map(|re| re.is_match(n))
            .unwrap_or(false)
        {
            signals.insert("Vite".into());
        }
    }

    if let Ok(pkg) = fs::read_to_string(root.join("package.json")) {
        let lower = pkg.to_ascii_lowercase();
        if lower.contains("\"next\"") {
            signals.insert("Next.js".into());
        }
        if lower.contains("\"vite\"") || lower.contains("\"@vitejs") {
            signals.insert("Vite".into());
        }
        if lower.contains("\"typescript\"") || lower.contains("\"ts-node\"") {
            signals.insert("TypeScript".into());
        }
        if lower.contains("\"react\"") {
            signals.insert("React".into());
        }
    }

    let mut out: Vec<String> = signals.into_iter().collect();
    out.sort();
    if out.is_empty() {
        out.push("Unknown / generic".into());
    }
    out
}

const README_CANDIDATES: &[&str] = &[
    "README.md",
    "Readme.md",
    "readme.md",
    "README.MD",
    "README.rst",
    "README",
];

const CONFIG_PRIORITY: &[&str] = &[
    "package.json",
    "pyproject.toml",
    "requirements.txt",
    "Cargo.toml",
    "tsconfig.json",
    "tauri.conf.json",
];

/// README-first: at most [`MAX_FILES_READ_CONTENT`] files, [`MAX_TOTAL_AI_CHARS`] total text.
fn select_readme_first_files(
    root: &Path,
    index: &[FileIndexEntry],
) -> Result<(Vec<SelectedFile>, bool, String), String> {
    let by_rel: std::collections::HashMap<String, &FileIndexEntry> =
        index.iter().map(|e| (e.rel_path.clone(), e)).collect();

    let mut ordered: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    fn push(ordered: &mut Vec<String>, seen: &mut HashSet<String>, path: &str) {
        if seen.insert(path.to_string()) {
            ordered.push(path.to_string());
        }
    }

    let mut readme_present = false;
    for p in README_CANDIDATES {
        if by_rel.contains_key(*p) {
            push(&mut ordered, &mut seen, p);
            readme_present = true;
            break;
        }
    }
    if !readme_present {
        for e in index {
            let base = Path::new(&e.rel_path)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("");
            let lower = base.to_ascii_lowercase();
            if lower == "readme.md" || lower == "readme.rst" || base == "README" {
                push(&mut ordered, &mut seen, e.rel_path.as_str());
                readme_present = true;
                break;
            }
        }
    }

    for p in CONFIG_PRIORITY {
        if by_rel.contains_key(*p) {
            push(&mut ordered, &mut seen, p);
        }
    }

    if ordered.len() < MAX_FILES_READ_CONTENT {
        let mut md_candidates: Vec<&FileIndexEntry> = index
            .iter()
            .filter(|e| e.extension == "md" || e.extension == "txt")
            .filter(|e| !README_CANDIDATES.contains(&e.rel_path.as_str()))
            .filter(|e| !is_minified_path(&e.rel_path))
            .collect();
        md_candidates.sort_by_key(|e| e.size);
        for e in md_candidates {
            if ordered.len() >= MAX_FILES_READ_CONTENT {
                break;
            }
            push(&mut ordered, &mut seen, e.rel_path.as_str());
        }
    }

    if ordered.len() > MAX_FILES_READ_CONTENT {
        ordered.truncate(MAX_FILES_READ_CONTENT);
    }

    let mut out: Vec<SelectedFile> = Vec::new();
    let mut total_chars = 0usize;
    let mut used_paths: Vec<String> = Vec::new();

    for rel in ordered {
        if is_lock_file(Path::new(&rel).file_name().and_then(|s| s.to_str()).unwrap_or("")) {
            continue;
        }
        if is_minified_path(&rel) {
            continue;
        }
        if let Some(content) = read_text_limited(root, &rel, MAX_SINGLE_FILE_CHARS) {
            let add = content.len().min(MAX_SINGLE_FILE_CHARS);
            if total_chars + add > MAX_TOTAL_AI_CHARS {
                break;
            }
            total_chars += content.len();
            used_paths.push(rel.clone());
            out.push(SelectedFile { rel_path: rel, content });
        }
        if out.len() >= MAX_FILES_READ_CONTENT {
            break;
        }
    }

    let notes = if used_paths.is_empty() {
        "No readable README or config files in the first two folder levels.".to_string()
    } else {
        format!(
            "README-first scan (depth≤{}): {} file(s), ~{} chars — {}",
            INDEX_MAX_DEPTH,
            used_paths.len(),
            total_chars,
            used_paths.join(", ")
        )
    };

    Ok((out, readme_present, notes))
}

pub fn scan_project(root: &Path) -> Result<ScanResult, String> {
    if !root.is_dir() {
        return Err("Selected path is not a folder".into());
    }

    let root_canon = fs::canonicalize(root).map_err(|e| e.to_string())?;
    let (index, index_truncated) = build_shallow_index(&root_canon)?;

    let detected = detect_stack(&root_canon, &index);
    let (selected_files, readme_present, scan_notes) = select_readme_first_files(&root_canon, &index)?;

    if selected_files.is_empty() {
        return Err(
            "No README or stack config found in the top folder levels. Add README.md (or package.json / Cargo.toml / pyproject.toml) at the project root, or pick a shallower folder."
                .into(),
        );
    }

    Ok(ScanResult {
        detected_stack: detected,
        file_index: index,
        index_truncated,
        selected_files,
        readme_present,
        scan_notes,
    })
}

pub fn index_sample_paths(entries: &[FileIndexEntry], limit: usize) -> Vec<String> {
    entries.iter().take(limit).map(|e| e.rel_path.clone()).collect()
}
