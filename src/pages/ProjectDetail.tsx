import { useCallback, useEffect, useState } from "react";
import { Link, useParams } from "react-router-dom";
import { invoke } from "@tauri-apps/api/core";
import type { ProjectDetail as PD } from "../types";

const projectDetailSessionCache = new Map<number, PD>();

function truncateAtWordBoundary(text: string, maxChars: number): string {
  const t = text.trim();
  if (t.length <= maxChars) return t;
  const slice = t.slice(0, maxChars);
  const lastSpace = slice.lastIndexOf(" ");
  if (lastSpace > Math.floor(maxChars * 0.55)) {
    return slice.slice(0, lastSpace).trimEnd();
  }
  return slice.trimEnd();
}

function ExpandableText({
  text,
  previewChars = 420,
  proseClass = "intel-prose",
}: {
  text: string | undefined | null;
  previewChars?: number;
  proseClass?: "intel-prose" | "intel-prose-wrap";
}) {
  const full = (text ?? "").trim();
  const [open, setOpen] = useState(false);
  if (!full) return null;
  const needMore = full.length > previewChars;
  const display = !needMore || open ? full : truncateAtWordBoundary(full, previewChars);
  const showEllipsis = needMore && !open;
  return (
    <div className="intel-expand">
      <p className={proseClass}>
        {display}
        {showEllipsis ? "…" : ""}
      </p>
      {needMore ? (
        <button type="button" className="intel-expand-toggle" onClick={() => setOpen((o) => !o)}>
          {open ? "Show less" : "Show more"}
        </button>
      ) : null}
    </div>
  );
}

function splitTalkingPoints(raw: string | undefined): string[] {
  if (!raw?.trim()) return [];
  const lines = raw
    .split(/\n+/)
    .map((l) => l.replace(/^[-•*]\s*/, "").trim())
    .filter(Boolean);
  if (lines.length > 1) return lines.slice(0, 6);
  const parts = raw.split(/(?<=[.!?])\s+/).map((s) => s.trim()).filter(Boolean);
  return parts.slice(0, 6);
}

export default function ProjectDetail() {
  const { id } = useParams();
  const [project, setProject] = useState<PD | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [reBusy, setReBusy] = useState(false);
  const [showDeep, setShowDeep] = useState(false);
  const [nameEdit, setNameEdit] = useState(false);
  const [nameDraft, setNameDraft] = useState("");
  const [renameBusy, setRenameBusy] = useState(false);

  const load = useCallback(async () => {
    if (!id) return;
    const numId = Number(id);
    setError(null);

    const cached = projectDetailSessionCache.get(numId);
    if (cached) {
      setProject(cached);
      return;
    }

    setLoading(true);
    try {
      const p = await invoke<PD | null>("get_project", { id: numId });
      setProject(p);
      if (p) projectDetailSessionCache.set(numId, p);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [id]);

  useEffect(() => {
    setProject(null);
    setShowDeep(false);
    setNameEdit(false);
    void load();
  }, [load]);

  useEffect(() => {
    if (project) setNameDraft(project.name);
  }, [project?.id, project?.name]);

  async function onReanalyze() {
    if (!id) return;
    setReBusy(true);
    setError(null);
    try {
      const p = await invoke<PD>("reanalyze_project", { id: Number(id) });
      projectDetailSessionCache.set(Number(id), p);
      setProject(p);
      window.dispatchEvent(new CustomEvent("peo:projects-changed"));
    } catch (e) {
      setError(String(e));
    } finally {
      setReBusy(false);
    }
  }

  async function onSaveRename() {
    if (!id || !project) return;
    const trimmed = nameDraft.trim();
    if (!trimmed) {
      setError("Name cannot be empty.");
      return;
    }
    setRenameBusy(true);
    setError(null);
    try {
      await invoke("rename_project", { id: Number(id), name: trimmed });
      const numId = Number(id);
      const next = { ...project, name: trimmed };
      projectDetailSessionCache.set(numId, next);
      setProject(next);
      setNameEdit(false);
      window.dispatchEvent(new CustomEvent("peo:projects-changed"));
    } catch (e) {
      setError(String(e));
    } finally {
      setRenameBusy(false);
    }
  }

  if (loading && !project) return <p className="muted">Loading project…</p>;
  if (!project) {
    return (
      <div>
        {error && <div className="error-banner">{error}</div>}
        <p className="muted">Project not found.</p>
        <Link to="/dashboard">← Dashboard</Link>
      </div>
    );
  }

  const a = project.analysis;
  const stack = (a?.tech_stack ?? project.detected_stack).slice(0, 8);
  const topInsights = (
    a?.core_features?.length
      ? a.core_features
      : [a?.one_line_summary ?? "", a?.problem_it_solves ?? "", a?.why_it_matters ?? ""]
  )
    .filter(Boolean)
    .slice(0, 5);

  const interviewPts = splitTalkingPoints(a?.interview_talking_points);

  return (
    <div className="project-detail-page">
      {error && <div className="error-banner">{error}</div>}

      <p style={{ margin: 0, display: "flex", flexWrap: "wrap", gap: "0.5rem", alignItems: "center" }}>
        <Link to="/dashboard">← Dashboard</Link>
        <Link to="/opportunities" className="btn">Opportunities</Link>
        <Link to="/case-study" className="btn">Case study</Link>
        <button type="button" className="btn" disabled={reBusy || !a} onClick={() => void onReanalyze()}>
          {reBusy ? "Re-analyzing…" : "Re-analyze"}
        </button>
      </p>

      <section className="card">
        <div className="project-detail-title-row">
          {nameEdit ? (
            <div className="project-detail-rename-row">
              <input
                type="text"
                value={nameDraft}
                maxLength={120}
                onChange={(e) => setNameDraft(e.target.value)}
                aria-label="Project name"
              />
              <button type="button" className="btn btn-primary" disabled={renameBusy} onClick={() => void onSaveRename()}>
                {renameBusy ? "Saving…" : "Save"}
              </button>
              <button
                type="button"
                className="btn"
                disabled={renameBusy}
                onClick={() => {
                  setNameDraft(project.name);
                  setNameEdit(false);
                }}
              >
                Cancel
              </button>
            </div>
          ) : (
            <>
              <h2 style={{ margin: 0, flex: "1 1 auto", minWidth: 0, wordBreak: "break-word" }}>{project.name}</h2>
              <button type="button" className="btn" onClick={() => setNameEdit(true)}>
                Rename
              </button>
            </>
          )}
        </div>
        {a?.positioning_label ? (
          <p className="meta" style={{ fontWeight: 600, marginTop: "0.25rem" }}>{a.positioning_label}</p>
        ) : null}
        <ExpandableText text={a?.one_line_summary ?? project.one_line_summary} previewChars={380} />
        <p className="meta" style={{ fontSize: "0.78rem", marginTop: "0.5rem", marginBottom: "0.25rem" }}>Stack</p>
        <div className="stack-tags">
          {stack.length ? stack.map((t) => (
            <span key={t} className="tag">{t}</span>
          )) : (
            <span className="muted" style={{ fontSize: "0.85rem" }}>—</span>
          )}
        </div>
      </section>

      <section className="card">
        <h3>What it actually does</h3>
        <ExpandableText
          text={a?.what_it_actually_does ?? a?.project_intent ?? a?.one_line_summary}
          previewChars={560}
        />
      </section>

      <section className="card">
        <h3>Problem & why it matters</h3>
        <p className="intel-prose" style={{ marginBottom: "0.35rem" }}><strong>Problem</strong></p>
        <ExpandableText text={a?.problem_it_solves} previewChars={480} />
        <p className="intel-prose" style={{ margin: "0.75rem 0 0.35rem" }}><strong>Value</strong></p>
        <ExpandableText text={a?.why_it_matters} previewChars={480} />
      </section>

      <section className="card">
        <h3>Core capabilities (3–5)</h3>
        <ul className="op-list">
          {topInsights.map((item, idx) => (
            <li key={idx} style={{ marginBottom: "0.35rem" }}>
              <ExpandableText text={item} previewChars={320} />
            </li>
          ))}
        </ul>
      </section>

      <section className="card">
        <h3>Interview narrative</h3>
        {interviewPts.length ? (
          <ul className="op-list">
            {interviewPts.map((line, idx) => (
              <li key={idx} style={{ marginBottom: "0.35rem" }}>
                <ExpandableText text={line} previewChars={400} proseClass="intel-prose-wrap" />
              </li>
            ))}
          </ul>
        ) : (
          <p className="muted">Re-analyze to generate interview lines.</p>
        )}
      </section>

      <section className="card">
        <h3>Positioning & portfolio</h3>
        {a?.portfolio_positioning?.trim() ? (
          <ExpandableText text={a.portfolio_positioning} previewChars={520} />
        ) : (
          <p className="muted">—</p>
        )}
      </section>

      <section className="card">
        <button type="button" className="btn" onClick={() => setShowDeep((v) => !v)}>
          {showDeep ? "Hide deep analysis" : "Deep analysis (technical)"}
        </button>
        {showDeep && a && (
          <div>
            <div className="intel-deep-section">
              <h4>Architecture</h4>
              <ExpandableText text={a.architecture_overview} previewChars={640} proseClass="intel-prose-wrap" />
            </div>
            <div className="intel-deep-section">
              <h4>Flow & structure</h4>
              <ExpandableText text={a.deep_explanation} previewChars={720} proseClass="intel-prose-wrap" />
            </div>
            <div className="intel-deep-section">
              <h4>Narrative</h4>
              <ExpandableText text={a.full_narrative_explanation} previewChars={640} proseClass="intel-prose-wrap" />
            </div>
            <div className="intel-deep-section">
              <h4>How it works (steps)</h4>
              <ul className="op-list">
                {(a.how_it_works_step_by_step ?? []).slice(0, 8).map((s, i) => (
                  <li key={i} style={{ marginBottom: "0.35rem" }}>
                    <ExpandableText text={s} previewChars={360} />
                  </li>
                ))}
              </ul>
            </div>
            <div className="intel-deep-section">
              <h4>Design decisions</h4>
              <ul className="op-list">
                {(a.design_decisions ?? []).slice(0, 6).map((s, i) => (
                  <li key={i} style={{ marginBottom: "0.35rem" }}>
                    <ExpandableText text={s} previewChars={360} />
                  </li>
                ))}
              </ul>
            </div>
            <div className="intel-deep-section">
              <h4>Tradeoffs & limitations</h4>
              <ul className="op-list">
                {(a.tradeoffs_and_limitations ?? []).slice(0, 5).map((s, i) => (
                  <li key={i} style={{ marginBottom: "0.35rem" }}>
                    <ExpandableText text={s} previewChars={360} />
                  </li>
                ))}
              </ul>
            </div>
            <div className="intel-deep-section">
              <h4>How to run</h4>
              {a.how_to_run?.trim() ? (
                <pre className="intel-code-block">{a.how_to_run}</pre>
              ) : (
                <p className="muted">—</p>
              )}
            </div>
            <p className="meta" style={{ fontSize: "0.78rem", marginTop: "0.75rem" }}>Full stack reference</p>
            <div className="stack-tags">
              {(a.tech_stack ?? []).length ? (a.tech_stack ?? []).map((t) => (
                <span key={t} className="tag">{t}</span>
              )) : (
                <span className="muted" style={{ fontSize: "0.85rem" }}>—</span>
              )}
            </div>
          </div>
        )}
      </section>
    </div>
  );
}
