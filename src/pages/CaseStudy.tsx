import { useCallback, useEffect, useMemo, useState } from "react";
import { Link } from "react-router-dom";
import { invoke } from "@tauri-apps/api/core";
import type {
  AiCaseStudyResult,
  CaseStudyPayload,
  ProjectListItem,
  UserProfile,
} from "../types";
import { isUserProfileFilled } from "../types";

function buildExportText(cs: CaseStudyPayload): string {
  const lines = [
    `# ${cs.title}`,
    "",
    "## Problem",
    cs.problem,
    "",
    "## Why it mattered",
    cs.why_it_mattered,
    "",
    "## Approach",
    cs.approach,
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
    "## Proof / examples (illustrative)",
    ...cs.proof_blocks.flatMap((b) => [
      `### ${b.title} (${b.kind})`,
      "```",
      b.body.trim(),
      "```",
      "",
    ]),
    "## LinkedIn (2 lines)",
    cs.linkedin_hook,
    "",
    "## One-liner",
    cs.quote_ready_one_liner,
  ];
  return lines.join("\n");
}

export default function CaseStudy() {
  const [projects, setProjects] = useState<ProjectListItem[]>([]);
  const [selectedId, setSelectedId] = useState<string>("");
  const [data, setData] = useState<CaseStudyPayload | null>(null);
  const [fromCache, setFromCache] = useState<boolean | null>(null);
  const [caseBusy, setCaseBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [copyMsg, setCopyMsg] = useState<string | null>(null);
  const [profile, setProfile] = useState<UserProfile | null>(null);

  const loadProjects = useCallback(async () => {
    setError(null);
    try {
      const rows = await invoke<ProjectListItem[]>("list_projects");
      setProjects(rows);
    } catch (e) {
      setError(String(e));
    }
  }, []);

  const loadProfile = useCallback(async () => {
    try {
      const p = await invoke<UserProfile | null>("get_user_profile");
      setProfile(p);
    } catch {
      setProfile(null);
    }
  }, []);

  useEffect(() => {
    void loadProjects();
    void loadProfile();
  }, [loadProjects, loadProfile]);

  const analyzedProjects = useMemo(
    () => projects.filter((p) => !!p.last_analyzed_at),
    [projects]
  );

  const profileReady = isUserProfileFilled(profile);

  const fetchCaseStudy = useCallback(
    async (regenerate: boolean) => {
      if (!selectedId) return;
      setCaseBusy(true);
      setError(null);
      setCopyMsg(null);
      if (!regenerate) {
        setData(null);
        setFromCache(null);
      }

      await new Promise<void>((r) => setTimeout(r, 0));

      try {
        const result = await invoke<AiCaseStudyResult>("generate_case_study", {
          id: Number(selectedId),
          regenerate,
        } satisfies { id: number; regenerate: boolean });
        setData(result.payload);
        setFromCache(result.from_cache);
      } catch (e) {
        setError(String(e));
      } finally {
        setCaseBusy(false);
      }
    },
    [selectedId]
  );

  useEffect(() => {
    if (!selectedId) {
      setData(null);
      setFromCache(null);
      setError(null);
      return;
    }
    setData(null);
    setFromCache(null);
    setError(null);
  }, [selectedId]);

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
        {" · "}
        <Link to="/setup">Writer profile</Link>
      </p>

      <h2>Case Study</h2>
      <p className="meta">
        Client-ready copy: problem, stakes, approach, outcome, plus illustrative
        proof (CLI / files / UI) inferred from your project—paste into proposals
        or LinkedIn.
      </p>

      {!profileReady && (
        <div className="profile-hint card">
          <p style={{ margin: 0 }}>
            <strong>Optional:</strong>{" "}
            <Link to="/setup">Set your writer profile</Link> so tone matches how
            you work (freelancer vs indie vs dev).
          </p>
        </div>
      )}

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
              style={{ minWidth: 320, padding: "0.5rem" }}
            >
              <option value="">Choose an analyzed project</option>
              {analyzedProjects.map((p) => (
                <option key={p.id} value={p.id}>
                  {p.name}
                </option>
              ))}
            </select>
            <div style={{ marginTop: "1rem", display: "flex", gap: "0.75rem", flexWrap: "wrap" }}>
              <button
                type="button"
                className="btn btn-primary"
                onClick={() => void fetchCaseStudy(false)}
                disabled={caseBusy || !selectedId}
              >
                {caseBusy && !data ? "Loading…" : "Load case study"}
              </button>
              <button
                type="button"
                onClick={() => void fetchCaseStudy(true)}
                disabled={caseBusy || !selectedId}
              >
                {caseBusy && !!data ? "Working…" : "Regenerate"}
              </button>
              {fromCache === true && !caseBusy && selectedId ? (
                <span className="muted" style={{ alignSelf: "center", fontSize: "0.85rem" }}>
                  Loaded from saved results
                </span>
              ) : null}
            </div>
            {caseBusy && selectedId ? (
              <p className="muted" style={{ marginTop: "0.75rem" }}>
                {data ? "Regenerating case study…" : "Loading case study…"}
              </p>
            ) : null}
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
            <span className="op-label">Why it mattered</span>
            <p className="op-body">{data.why_it_mattered}</p>
          </div>
          <div className="op-field">
            <span className="op-label">Approach</span>
            <p className="op-body">{data.approach}</p>
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
            <span className="op-label">Proof & examples</span>
            <p className="meta" style={{ marginBottom: "0.5rem" }}>
              Illustrative outputs inferred from your project—grounded in stack and
              behavior, not screenshots.
            </p>
            <div className="proof-blocks">
              {data.proof_blocks.map((b, i) => (
                <div key={i} className="proof-block card">
                  <div className="proof-block-head">
                    <span className="proof-kind">{b.kind}</span>
                    <strong>{b.title}</strong>
                  </div>
                  <pre className="proof-body">{b.body}</pre>
                </div>
              ))}
            </div>
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
