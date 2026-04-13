use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

const MAX_FILE_CHARS: usize = 40_000;
const MAX_INDEX_PATHS: usize = 400;
const MAX_TOTAL_PAYLOAD_FILES: usize = 18;

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

fn read_text_limited(root: &Path, rel: &str) -> Option<String> {
    let full = root.join(rel);
    let bytes = fs::read(&full).ok()?;
    if bytes.len() > 2_000_000 {
        return None;
    }
    let mut s = String::from_utf8_lossy(&bytes).into_owned();
    if s.lines().take(50).any(|l| l.len() > 8000) {
        return None;
    }
    if s.len() > MAX_FILE_CHARS {
        s.truncate(MAX_FILE_CHARS);
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

pub fn scan_project(root: &Path) -> Result<ScanResult, String> {
    if !root.is_dir() {
        return Err("Selected path is not a folder".into());
    }

    let root_canon = fs::canonicalize(root).map_err(|e| e.to_string())?;
    let mut index: Vec<FileIndexEntry> = Vec::new();

    for entry in WalkDir::new(&root_canon).follow_links(false).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let rel = path
            .strip_prefix(&root_canon)
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
        let size = meta.len();
        index.push(FileIndexEntry {
            rel_path: rel,
            extension: ext,
            size,
        });
    }

    index.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
    let index_truncated = index.len() > MAX_INDEX_PATHS;
    if index.len() > MAX_INDEX_PATHS {
        index.truncate(MAX_INDEX_PATHS);
    }

    let detected = detect_stack(&root_canon, &index);
    let selected = select_files(&root_canon, &index)?;

    Ok(ScanResult {
        detected_stack: detected,
        file_index: index,
        index_truncated,
        selected_files: selected,
    })
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

fn select_files(root: &Path, index: &[FileIndexEntry]) -> Result<Vec<SelectedFile>, String> {
    let by_rel: HashMap<String, &FileIndexEntry> = index.iter().map(|e| (e.rel_path.clone(), e)).collect();

    let mut ordered: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    fn push(ordered: &mut Vec<String>, seen: &mut HashSet<String>, path: &str) {
        if seen.insert(path.to_string()) {
            ordered.push(path.to_string());
        }
    }

    for p in [
        "README.md",
        "Readme.md",
        "readme.md",
        "README.rst",
        "README",
    ] {
        if by_rel.contains_key(p) {
            push(&mut ordered, &mut seen, p);
            break;
        }
    }

    let config_patterns: &[&str] = &[
        "package.json",
        "requirements.txt",
        "pyproject.toml",
        "Cargo.toml",
        "tsconfig.json",
        "tauri.conf.json",
        ".env.example",
        "docker-compose.yml",
        "Dockerfile",
    ];
    for p in config_patterns {
        if by_rel.contains_key(*p) {
            push(&mut ordered, &mut seen, p);
        }
    }

    for e in index {
        if e.rel_path.ends_with("prisma/schema.prisma") {
            push(&mut ordered, &mut seen, e.rel_path.as_str());
        }
    }

    for e in index {
        let n = e.rel_path.as_str();
        if Regex::new(r"(?i)(^|/)vite\.config\.(ts|js|mts|cts)$")
            .ok()
            .map(|re| re.is_match(n))
            .unwrap_or(false)
        {
            push(&mut ordered, &mut seen, n);
        }
        if Regex::new(r"(?i)(^|/)next\.config\.(js|mjs|cjs|ts)$")
            .ok()
            .map(|re| re.is_match(n))
            .unwrap_or(false)
        {
            push(&mut ordered, &mut seen, n);
        }
    }

    let entry_names = [
        "main.ts",
        "main.tsx",
        "main.js",
        "main.jsx",
        "App.tsx",
        "App.jsx",
        "App.vue",
        "main.py",
        "app.py",
        "index.js",
        "index.ts",
        "index.tsx",
        "server.js",
        "server.ts",
    ];

    for e in index {
        let file = Path::new(&e.rel_path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        if entry_names.contains(&file) {
            push(&mut ordered, &mut seen, e.rel_path.as_str());
        }
    }

    for folder in ["src", "app", "lib", "components", "pages", "backend", "api"] {
        let mut candidates: Vec<&FileIndexEntry> = index
            .iter()
            .filter(|e| {
                e.rel_path.starts_with(&format!("{}/", folder))
                    || e.rel_path.starts_with(&format!("{}/", folder.to_ascii_uppercase()))
            })
            .filter(|e| {
                let ext = e.extension.as_str();
                matches!(ext, "ts" | "tsx" | "js" | "jsx" | "vue" | "py" | "rs" | "go" | "java" | "kt")
            })
            .filter(|e| !is_minified_path(&e.rel_path))
            .filter(|e| !is_lock_file(
                Path::new(&e.rel_path)
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or(""),
            ))
            .collect();
        candidates.sort_by_key(|e| e.size);
        for c in candidates.iter().take(4) {
            push(&mut ordered, &mut seen, c.rel_path.as_str());
        }
    }

    if ordered.len() > MAX_TOTAL_PAYLOAD_FILES {
        ordered.truncate(MAX_TOTAL_PAYLOAD_FILES);
    }

    let mut final_paths = ordered;

    if final_paths.is_empty() {
        let mut rest: Vec<&FileIndexEntry> = index
            .iter()
            .filter(|e| {
                let ext = e.extension.as_str();
                matches!(ext, "md" | "txt" | "ts" | "tsx" | "js" | "jsx" | "py" | "rs" | "go")
            })
            .filter(|e| !is_lock_file(
                Path::new(&e.rel_path)
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or(""),
            ))
            .filter(|e| !is_minified_path(&e.rel_path))
            .collect();
        rest.sort_by_key(|e| e.size);
        for e in rest.iter().take(6) {
            final_paths.push(e.rel_path.clone());
        }
    }

    let mut out: Vec<SelectedFile> = Vec::new();
    for rel in final_paths {
        if is_lock_file(Path::new(&rel).file_name().and_then(|s| s.to_str()).unwrap_or("")) {
            continue;
        }
        if is_minified_path(&rel) {
            continue;
        }
        if let Some(content) = read_text_limited(root, &rel) {
            out.push(SelectedFile { rel_path: rel, content });
        }
    }

    Ok(out)
}

pub fn index_sample_paths(entries: &[FileIndexEntry], limit: usize) -> Vec<String> {
    entries.iter().take(limit).map(|e| e.rel_path.clone()).collect()
}
