import { memo, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Link } from "react-router-dom";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type { ProjectListItem, RuntimeStatus, TopProjectsPayload } from "../types";

const IMPORT_STEPS = [
  "Reading README & config files…",
  "Detecting stack…",
  "Generating explanation…",
  "Saving project…",
];

const ProjectCard = memo(function ProjectCard({
  p,
  onTogglePin,
}: {
  p: ProjectListItem;
  onTogglePin: (id: number, nextPinned: boolean) => void;
}) {
  return (
    <div className="card" style={{ display: "grid", gap: "0.5rem" }}>
      <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", gap: "0.5rem" }}>
        <h2 style={{ margin: 0, fontSize: "1.05rem" }}>
          {p.name} {p.is_pinned ? "★" : ""}
        </h2>
        <button
          type="button"
          className="btn"
          onClick={() => onTogglePin(p.id, !p.is_pinned)}
          title={p.is_pinned ? "Unpin project" : "Pin project"}
        >
          {p.is_pinned ? "Unpin" : "Pin"}
        </button>
      </div>
      <p className="meta" style={{ margin: 0, fontSize: "0.92rem" }}>{p.one_line_summary || "—"}</p>
      <p className="meta" style={{ margin: 0, fontSize: "0.92rem" }}>
        Last analyzed: {p.last_analyzed_at ? new Date(p.last_analyzed_at).toLocaleString() : "—"}
      </p>
      <div>
        <Link to={`/project/${p.id}`} className="btn btn-primary">
          Open Project
        </Link>
      </div>
    </div>
  );
});

export default function Dashboard() {
  const [projects, setProjects] = useState<ProjectListItem[]>([]);
  const [query, setQuery] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [rankBusy, setRankBusy] = useState(false);
  const [rankError, setRankError] = useState<string | null>(null);
  const [topPicks, setTopPicks] = useState<TopProjectsPayload | null>(null);
  const [status, setStatus] = useState<RuntimeStatus | null>(null);
  const [importStep, setImportStep] = useState<string | null>(null);
  const importStepTimer = useRef<ReturnType<typeof setInterval> | null>(null);

  const load = useCallback(async () => {
    try {
      const list = await invoke<ProjectListItem[]>("list_projects");
      setProjects(list);
    } catch (e) {
      setError(String(e));
    }
  }, []);

  useEffect(() => {
    const onProjectsChanged = () => void load();
    window.addEventListener("peo:projects-changed", onProjectsChanged);
    return () => window.removeEventListener("peo:projects-changed", onProjectsChanged);
  }, [load]);

  useEffect(() => {
    setError(null);
    void load();
    void (async () => {
      try {
        const s = await invoke<RuntimeStatus>("get_runtime_status");
        setStatus(s);
      } catch {
        setStatus(null);
      }
    })();
  }, [load]);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    const base = !q
      ? projects
      : projects.filter((p) => {
          return p.name.toLowerCase().includes(q) || p.one_line_summary.toLowerCase().includes(q);
        });
    return [...base].sort((a, b) => {
      if (a.is_pinned !== b.is_pinned) return a.is_pinned ? -1 : 1;
      const da = a.last_analyzed_at ? Date.parse(a.last_analyzed_at) : 0;
      const db = b.last_analyzed_at ? Date.parse(b.last_analyzed_at) : 0;
      return db - da;
    });
  }, [projects, query]);

  async function onImport() {
    setError(null);
    const selected = await open({
      directory: true,
      multiple: false,
      title: "Choose project folder",
    });
    if (!selected || Array.isArray(selected)) return;
    setBusy(true);
    let stepIdx = 0;
    setImportStep(IMPORT_STEPS[0]);
    importStepTimer.current = setInterval(() => {
      stepIdx = (stepIdx + 1) % IMPORT_STEPS.length;
      setImportStep(IMPORT_STEPS[stepIdx]);
    }, 850);
    try {
      await new Promise<void>((r) => setTimeout(r, 0));
      await invoke("import_project", { path: selected });
      await load();
    } catch (e) {
      setError(String(e));
    } finally {
      if (importStepTimer.current) {
        clearInterval(importStepTimer.current);
        importStepTimer.current = null;
      }
      setImportStep(null);
      setBusy(false);
    }
  }

  async function onTogglePin(id: number, nextPinned: boolean) {
    setProjects((prev) => prev.map((p) => (p.id === id ? { ...p, is_pinned: nextPinned } : p)));
    try {
      await invoke("toggle_project_pin", { id, pinned: nextPinned });
    } catch (e) {
      setProjects((prev) => prev.map((p) => (p.id === id ? { ...p, is_pinned: !nextPinned } : p)));
      setError(String(e));
    }
  }

  async function refreshTopPicks() {
    setRankBusy(true);
    setRankError(null);
    try {
      const r = await invoke<TopProjectsPayload>("rank_top_projects");
      setTopPicks(r);
    } catch (e) {
      setTopPicks(null);
      setRankError(String(e));
    } finally {
      setRankBusy(false);
    }
  }

  return (
    <div>
      {error && <div className="error-banner">{error}</div>}
      {busy && importStep && (
        <div className="card import-progress" style={{ marginBottom: "1rem" }}>
          <strong>Import in progress</strong>
          <p className="meta" style={{ margin: "0.35rem 0 0" }}>{importStep}</p>
          <p className="muted" style={{ margin: "0.35rem 0 0", fontSize: "0.85rem" }}>
            README-first scan (depth ≤2, max 20 files). Large dependency folders are skipped.
          </p>
        </div>
      )}
      {status && !status.hasApiKey && (
        <div className="error-banner">
          API key missing. Add keys in <Link to="/settings">Settings</Link> or set ANTHROPIC_API_KEY / OPENAI_API_KEY in the environment.
        </div>
      )}

      <section className="card" style={{ marginBottom: "1rem" }}>
        <strong>Quick stats</strong>
        <p className="meta" style={{ marginTop: "0.5rem" }}>
          Projects: {projects.length} · Pinned: {projects.filter((p) => p.is_pinned).length}
        </p>
      </section>

      <section className="top-picks-section card" style={{ marginBottom: "1rem" }}>
        <div className="top-picks-head">
          <h2 style={{ margin: "0 0 0.35rem", fontSize: "1.1rem" }}>Top picks</h2>
          <button type="button" className="btn" disabled={rankBusy} onClick={() => void refreshTopPicks()}>
            {rankBusy ? "Refreshing..." : "Refresh ranking"}
          </button>
        </div>
        {rankError && <p className="muted">{rankError}</p>}
        {topPicks?.picks?.length ? (
          <ul className="top-picks-list">
            {topPicks.picks.map((p, i) => (
              <li key={p.project_id}>
                <Link to={`/project/${p.project_id}`}><strong>#{i + 1} {p.project_name}</strong></Link>
                <p className="meta" style={{ margin: "0.25rem 0 0" }}>{p.rationale}</p>
              </li>
            ))}
          </ul>
        ) : (
          !rankBusy && <p className="muted">No ranking yet.</p>
        )}
      </section>

      <div style={{ display: "flex", flexWrap: "wrap", gap: "0.75rem", alignItems: "center", marginBottom: "1rem" }}>
        <Link to="/" className="btn">Home</Link>
        <button type="button" className="btn btn-primary" onClick={() => void onImport()} disabled={busy}>
          {busy ? "Importing..." : "Import Project"}
        </button>
        <Link to="/setup" className="btn">Profile</Link>
        <Link to="/case-study" className="btn">Case Study</Link>
        <Link to="/opportunities" className="btn">Opportunities</Link>
        <Link to="/idea-projects" className="btn">Ideas</Link>
        <input
          className="search"
          type="search"
          placeholder="Search projects..."
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          aria-label="Filter projects"
        />
      </div>

      <div className="card-grid">
        {filtered.length === 0 ? (
          <p className="muted">No projects yet. Import a folder to start.</p>
        ) : (
          filtered.map((p) => <ProjectCard key={p.id} p={p} onTogglePin={onTogglePin} />)
        )}
      </div>
    </div>
  );
}
