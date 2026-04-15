import { memo, useCallback, useEffect, useMemo, useState } from "react";
import { Link } from "react-router-dom";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type { ProjectListItem, RuntimeStatus, TopProjectsPayload } from "../types";

const ProjectCard = memo(function ProjectCard({ p }: { p: ProjectListItem }) {
  return (
    <Link to={`/project/${p.id}`} style={{ color: "inherit" }}>
      <div className="card">
        <h2>{p.name}</h2>
        <p className="meta">{p.one_line_summary || "—"}</p>
        <p className="meta">
          Last analyzed:{" "}
          {p.last_analyzed_at
            ? new Date(p.last_analyzed_at).toLocaleString()
            : "—"}
        </p>
      </div>
    </Link>
  );
});

export default function Dashboard() {
  const [projects, setProjects] = useState<ProjectListItem[]>([]);
  const [query, setQuery] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [topPicks, setTopPicks] = useState<TopProjectsPayload | null>(null);
  const [rankBusy, setRankBusy] = useState(false);
  const [rankError, setRankError] = useState<string | null>(null);
  const [status, setStatus] = useState<RuntimeStatus | null>(null);

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

  useEffect(() => {
    void (async () => {
      try {
        const s = await invoke<RuntimeStatus>("get_runtime_status");
        setStatus(s);
      } catch {
        setStatus(null);
      }
    })();
  }, []);

  const refreshTopPicks = useCallback(async () => {
    setRankError(null);
    setRankBusy(true);
    try {
      const r = await invoke<TopProjectsPayload>("rank_top_projects");
      setTopPicks(r);
    } catch (e) {
      setRankError(String(e));
      setTopPicks(null);
    } finally {
      setRankBusy(false);
    }
  }, []);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return projects;
    return projects.filter((p) => {
      return (
        p.name.toLowerCase().includes(q) ||
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
      {status && !status.hasApiKey && (
        <div className="error-banner">
          Missing API key. Add it before analysis/generation. You can still browse local projects.
        </div>
      )}
      {status && !status.hasProfile && (
        <p className="muted" style={{ marginBottom: "0.75rem" }}>
          Tip: set your profile in Setup for better tone.
        </p>
      )}

      <section className="card" style={{ marginBottom: "1rem" }}>
        <strong>Quick stats</strong>
        <p className="meta" style={{ marginTop: "0.5rem" }}>
          Projects: {projects.length} · Analyzed: {projects.filter((p) => !!p.last_analyzed_at).length}
        </p>
      </section>

      <section className="top-picks-section card" style={{ marginBottom: "1.25rem" }}>
        <div className="top-picks-head">
          <h2 style={{ margin: "0 0 0.35rem", fontSize: "1.05rem" }}>
            Top picks for your goal
          </h2>
          <button
            type="button"
            className="btn"
            disabled={rankBusy}
            onClick={() => void refreshTopPicks()}
          >
            {rankBusy ? "Refreshing…" : "Refresh ranking"}
          </button>
        </div>
        <p className="meta" style={{ marginTop: 0 }}>
          AI picks up to 3 analyzed projects that best fit your profile goal
          (e.g. client work → easiest to explain and sell).
        </p>
        {rankError && (
          <p className="muted" style={{ color: "#e8a0a0" }}>
            {rankError}
          </p>
        )}
        {topPicks && topPicks.picks.length > 0 ? (
          <ul className="top-picks-list">
            {topPicks.picks.map((p, i) => (
              <li key={p.project_id}>
                <Link to={`/project/${p.project_id}`}>
                  <strong>
                    #{i + 1} {p.project_name}
                  </strong>
                </Link>
                <p className="meta" style={{ margin: "0.25rem 0 0" }}>
                  {p.rationale}
                </p>
              </li>
            ))}
          </ul>
        ) : (
          !rankError &&
          !rankBusy && (
            <p className="muted">No ranking yet — analyze a project first.</p>
          )
        )}
      </section>
      <div
        style={{
          display: "flex",
          flexWrap: "wrap",
          gap: "0.75rem",
          alignItems: "center",
          marginBottom: "1.25rem",
        }}
      >
        <Link to="/" className="btn">
          Home
        </Link>
        <button
          type="button"
          className="btn btn-primary"
          onClick={() => void onImport()}
          disabled={busy}
        >
          {busy ? "Importing…" : "Import Project"}
        </button>
        <Link to="/setup" className="btn">
          Profile
        </Link>
        <Link to="/case-study" className="btn">
          Case Study
        </Link>
        <Link to="/opportunities" className="btn">
          Opportunities dashboard
        </Link>
        <Link to="/idea-projects" className="btn">
          Idea Projects
        </Link>
        <input
          className="search"
          type="search"
          placeholder="Search by name or summary…"
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
          <ProjectCard key={p.id} p={p} />
        ))}
      </div>
    </div>
  );
}
