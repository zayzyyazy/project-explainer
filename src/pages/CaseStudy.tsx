import { useEffect, useMemo, useState } from "react";
import { Link } from "react-router-dom";
import { invoke } from "@tauri-apps/api/core";
import type { CaseStudyPayload, ProjectRow } from "../types";

function buildExportText(cs: CaseStudyPayload): string {
  const lines = [
    `# ${cs.title}`,
    "",
    "## Problem",
    cs.problem,
    "",
    "## Solution",
    cs.solution,
    "",
    "## Outcome",
    cs.outcome,
    "",
    `_Outcome basis: ${cs.outcome_basis}_`,
    "",
    "## Narrative",
    cs.narrative,
    "",
    "## What we built",
    ...cs.what_we_built.map((x) => `- ${x}`),
    "",
    "## LinkedIn (2 lines)",
    cs.linkedin_hook,
    "",
    "## One-liner",
    cs.quote_ready_one_liner,
  ];
  return lines.join("\n");
}

export default function CaseStudy() {
  const [projects, setProjects] = useState<ProjectRow[]>([]);
  const [selectedId, setSelectedId] = useState<string>("");
  const [data, setData] = useState<CaseStudyPayload | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [copyMsg, setCopyMsg] = useState<string | null>(null);

  useEffect(() => {
    void loadProjects();
  }, []);

  async function loadProjects() {
    setError(null);
    try {
      const rows = await invoke<ProjectRow[]>("list_projects");
      setProjects(rows);
    } catch (e) {
      setError(String(e));
    }
  }

  const analyzedProjects = useMemo(
    () => projects.filter((p) => !!p.last_analyzed_at),
    [projects]
  );

  async function onGenerate() {
    if (!selectedId) return;
    setBusy(true);
    setError(null);
    setData(null);
    setCopyMsg(null);
    try {
      const result = await invoke<CaseStudyPayload>("generate_case_study", {
        id: Number(selectedId),
      });
      setData(result);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  async function copyAll() {
    if (!data) return;
    try {
      await navigator.clipboard.writeText(buildExportText(data));
      setCopyMsg("Copied to clipboard.");
      setTimeout(() => setCopyMsg(null), 2500);
    } catch {
      setCopyMsg("Could not copy — select text manually.");
    }
  }

  return (
    <div>
      <p>
        <Link to="/dashboard">← Dashboard</Link>
      </p>

      <h2>Case Study</h2>
      <p className="meta">
        One client-winning story: problem, solution, outcome, and paste-ready
        copy — grounded in your analyzed project.
      </p>

      {error && <div className="error-banner">{error}</div>}

      <section className="detail-section">
        <h3>Project</h3>
        {analyzedProjects.length === 0 ? (
          <p className="muted">Import and analyze a project on the Dashboard first.</p>
        ) : (
          <>
            <select
              value={selectedId}
              onChange={(e) => setSelectedId(e.target.value)}
              disabled={busy}
              style={{ minWidth: 320, padding: "0.5rem" }}
            >
              <option value="">Choose an analyzed project</option>
              {analyzedProjects.map((p) => (
                <option key={p.id} value={p.id}>
                  {p.name}
                </option>
              ))}
            </select>
            <div style={{ marginTop: "1rem" }}>
              <button
                type="button"
                className="btn btn-primary"
                onClick={() => void onGenerate()}
                disabled={busy || !selectedId}
              >
                {busy ? "Generating…" : "Generate case study"}
              </button>
            </div>
          </>
        )}
      </section>

      {copyMsg && <p className="muted">{copyMsg}</p>}

      {data && (
        <article className="card case-study-output">
          <div className="case-study-actions">
            <button type="button" className="btn" onClick={() => void copyAll()}>
              Copy all (Markdown)
            </button>
          </div>

          <h3 className="case-study-title">{data.title}</h3>

          <div className="op-field">
            <span className="op-label">Problem</span>
            <p className="op-body">{data.problem}</p>
          </div>
          <div className="op-field">
            <span className="op-label">Solution</span>
            <p className="op-body">{data.solution}</p>
          </div>
          <div className="op-field">
            <span className="op-label">Outcome</span>
            <p className="op-body">{data.outcome}</p>
            <p className="meta" style={{ marginTop: "0.35rem" }}>
              Basis: {data.outcome_basis}
            </p>
          </div>

          <div className="op-field">
            <span className="op-label">Narrative</span>
            <p className="op-body narrative-block">{data.narrative}</p>
          </div>

          <div className="op-field">
            <span className="op-label">What we built</span>
            <ul className="op-list">
              {data.what_we_built.map((x, i) => (
                <li key={i}>{x}</li>
              ))}
            </ul>
          </div>

          <div className="op-field">
            <span className="op-label">LinkedIn hook</span>
            <p className="op-body">{data.linkedin_hook}</p>
          </div>

          <div className="op-field">
            <span className="op-label">Quote-ready one-liner</span>
            <p className="op-body">{data.quote_ready_one_liner}</p>
          </div>
        </article>
      )}
    </div>
  );
}
