import { useCallback, useEffect, useMemo, useState } from "react";
import { Link } from "react-router-dom";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type { ProjectListItem } from "../types";

export default function Dashboard() {
  const [projects, setProjects] = useState<ProjectListItem[]>([]);
  const [query, setQuery] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const load = useCallback(async () => {
    setError(null);
    try {
      const list = await invoke<ProjectListItem[]>("list_projects");
      setProjects(list);
    } catch (e) {
      setError(String(e));
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return projects;
    return projects.filter((p) => {
      const stack = p.detected_stack.join(" ").toLowerCase();
      return (
        p.name.toLowerCase().includes(q) ||
        stack.includes(q) ||
        (p.one_line_summary || "").toLowerCase().includes(q)
      );
    });
  }, [projects, query]);

  async function onImport() {
    setError(null);
    const selected = await open({
      directory: true,
      multiple: false,
      title: "Choose project folder",
    });
    if (selected === null || Array.isArray(selected)) return;
    setBusy(true);
    try {
      await invoke("import_project", { path: selected });
    } catch (e) {
      setError(String(e));
    } finally {
      await load();
      setBusy(false);
    }
  }

  return (
    <div>
      {error && <div className="error-banner">{error}</div>}
      <div
        style={{
          display: "flex",
          flexWrap: "wrap",
          gap: "0.75rem",
          alignItems: "center",
          marginBottom: "1.25rem",
        }}
      >
        <button
          type="button"
          className="btn btn-primary"
          onClick={() => void onImport()}
          disabled={busy}
        >
          {busy ? "Importing…" : "Import Project"}
        </button>
        <input
          className="search"
          type="search"
          placeholder="Search by name or stack…"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          aria-label="Filter projects"
        />
        <span className="muted">{filtered.length} project(s)</span>
      </div>

      <div className="card-grid">
        {filtered.length === 0 && (
          <p className="muted">
            No projects yet. Import a folder to analyze it with Claude.
          </p>
        )}
        {filtered.map((p) => (
          <Link key={p.id} to={`/project/${p.id}`} style={{ color: "inherit" }}>
            <div className="card">
              <h2>{p.name}</h2>
              <p className="meta">{p.one_line_summary || "—"}</p>
              <p className="meta" title={p.path}>
                {p.path}
              </p>
              <p className="meta">
                Last analyzed:{" "}
                {p.last_analyzed_at
                  ? new Date(p.last_analyzed_at).toLocaleString()
                  : "—"}
              </p>
              <div className="stack-tags">
                {p.detected_stack.map((s) => (
                  <span key={s} className="tag">
                    {s}
                  </span>
                ))}
              </div>
            </div>
          </Link>
        ))}
      </div>
    </div>
  );
}
