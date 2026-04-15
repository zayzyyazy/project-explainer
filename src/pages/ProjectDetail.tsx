import { useCallback, useEffect, useMemo, useState } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";
import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import type {
  EvolutionSuggestionsPayload,
  ExportBundleResult,
  IncrementalUpdateResult,
  LinkedinResult,
  PositioningPayload,
  ProjectImportancePayload,
  ProjectDetail as PD,
} from "../types";

/** Avoid re-fetching `get_project` when revisiting the same id in one app session. */
const projectDetailSessionCache = new Map<number, PD>();

const NARRATIVE_PREVIEW_CHARS = 150;

export default function ProjectDetail() {
  const { id } = useParams();
  const navigate = useNavigate();

  const [project, setProject] = useState<PD | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [positioning, setPositioning] = useState<PositioningPayload | null>(null);
  const [evolutionIdeas, setEvolutionIdeas] =
    useState<EvolutionSuggestionsPayload | null>(null);
  const [insightError, setInsightError] = useState<string | null>(null);
  const [updateMsg, setUpdateMsg] = useState<string | null>(null);
  const [narrativeExpanded, setNarrativeExpanded] = useState(false);
  const [importance, setImportance] = useState<ProjectImportancePayload | null>(null);
  const [linkedinText, setLinkedinText] = useState<string>("");
  const [linkedinLength, setLinkedinLength] = useState<"short" | "long">("short");
  const [linkedinFocus, setLinkedinFocus] = useState<
    "describe_tool" | "describe_stack" | "describe_problem" | "describe_outcome"
  >("describe_outcome");
  const [collapsed, setCollapsed] = useState({
    intelligence: true,
    narrative: true,
    details: true,
  });

  const load = useCallback(async (force = false) => {
    if (!id) return;
    const numId = Number(id);
    setError(null);
    if (!force) {
      const cached = projectDetailSessionCache.get(numId);
      if (cached) {
        setProject(cached);
        return;
      }
    }
    try {
      const p = await invoke<PD | null>("get_project", {
        id: numId,
      });
      setProject(p);
      if (p) projectDetailSessionCache.set(numId, p);
    } catch (e) {
      setError(String(e));
    }
  }, [id]);

  useEffect(() => {
    setPositioning(null);
    setEvolutionIdeas(null);
    setInsightError(null);
    setUpdateMsg(null);
    setNarrativeExpanded(false);
    setImportance(null);
    setLinkedinText("");
    setCollapsed({ intelligence: true, narrative: true, details: true });
    setProject(null);
    void load(false);
    void (async () => {
      if (!id) return;
      try {
        const payload = await invoke<ProjectImportancePayload>("get_project_importance", {
          id: Number(id),
        });
        setImportance(payload);
      } catch {
        setImportance(null);
      }
    })();
  }, [id, load]);

  async function onReanalyze() {
    if (!id) return;
    const numId = Number(id);
    setBusy(true);
    setError(null);
    try {
      projectDetailSessionCache.delete(numId);
      await invoke("reanalyze_project", { id: numId });
      await load(true);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  async function onDelete() {
    if (!id) return;
    if (!window.confirm("Delete this project from the library?")) return;
    setBusy(true);
    setError(null);
    try {
      projectDetailSessionCache.delete(Number(id));
      await invoke("delete_project", { id: Number(id) });
      navigate("/dashboard");
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  async function onIncrementalUpdate() {
    if (!id) return;
    setBusy(true);
    setInsightError(null);
    setUpdateMsg(null);
    try {
      const res = await invoke<IncrementalUpdateResult>("incremental_project_update", {
        id: Number(id),
      });
      setUpdateMsg(
        `Recorded: ${res.payload.version_label} — ${res.payload.what_changed_overview.slice(0, 120)}…`
      );
      projectDetailSessionCache.delete(Number(id));
      await load(true);
    } catch (e) {
      setInsightError(String(e));
    } finally {
      setBusy(false);
    }
  }

  async function onLoadPositioning() {
    if (!id) return;
    setBusy(true);
    setInsightError(null);
    try {
      const p = await invoke<PositioningPayload>("get_positioning_clarity", {
        id: Number(id),
      });
      setPositioning(p);
    } catch (e) {
      setInsightError(String(e));
    } finally {
      setBusy(false);
    }
  }

  async function onLoadEvolutionSuggestions() {
    if (!id) return;
    setBusy(true);
    setInsightError(null);
    try {
      const p = await invoke<EvolutionSuggestionsPayload>("suggest_evolution_steps", {
        id: Number(id),
      });
      setEvolutionIdeas(p);
    } catch (e) {
      setInsightError(String(e));
    } finally {
      setBusy(false);
    }
  }

  async function onExportMd() {
    if (!project?.analysis) return;

    const path = await save({
      defaultPath: `${project.analysis.project_name.replace(/[^\w.-]+/g, "_")}.md`,
      filters: [{ name: "Markdown", extensions: ["md"] }],
      title: "Export summary as Markdown",
    });

    if (!path) return;

    setBusy(true);
    setError(null);

    try {
      await invoke("export_markdown", {
        id: Number(id),
        filePath: path,
      });
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  async function onGenerateLinkedin() {
    if (!id) return;
    setBusy(true);
    setInsightError(null);
    try {
      const res = await invoke<LinkedinResult>("generate_linkedin_post", {
        id: Number(id),
        length: linkedinLength,
        focus: linkedinFocus,
      });
      setLinkedinText(res.text);
    } catch (e) {
      setInsightError(String(e));
    } finally {
      setBusy(false);
    }
  }

  async function onExportBundle() {
    if (!id) return;
    const selected = await open({
      directory: true,
      multiple: false,
      title: "Choose export folder",
    });
    if (!selected || Array.isArray(selected)) return;
    setBusy(true);
    setError(null);
    try {
      const res = await invoke<ExportBundleResult>("export_project_bundle", {
        id: Number(id),
        outputDir: selected,
        includeOpportunities: true,
      });
      setUpdateMsg(`Exported: ${res.writtenFiles.join(", ")}`);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  const narrativePreview = useMemo(() => {
    const text = project?.analysis?.full_narrative_explanation;
    if (!text) return null;
    if (narrativeExpanded || text.length <= NARRATIVE_PREVIEW_CHARS) return text;
    return `${text.slice(0, NARRATIVE_PREVIEW_CHARS)}…`;
  }, [project?.analysis?.full_narrative_explanation, narrativeExpanded]);

  if (!project && !error) return <p className="muted">Loading…</p>;

  if (!project) {
    return (
      <div>
        {error && <div className="error-banner">{error}</div>}
        <p>Project not found.</p>
        <Link to="/dashboard">← Back</Link>
      </div>
    );
  }

  const a = project.analysis;

  return (
    <div>
      {error && <div className="error-banner">{error}</div>}

      <p>
        <Link to="/dashboard">← Dashboard</Link>
      </p>

      <h2>{a?.project_name ?? project.name}</h2>

      <p className="meta">{project.path}</p>

      <div className="stack-tags">
        {(a?.tech_stack ?? project.detected_stack).map((s) => (
          <span key={s} className="tag">
            {s}
          </span>
        ))}
      </div>

      <div className="actions">
        <button onClick={onReanalyze} disabled={busy}>
          {busy ? "Working…" : "Re-analyze"}
        </button>
        <button
          type="button"
          onClick={() => void onIncrementalUpdate()}
          disabled={busy || !a}
          title="Scan folder vs last analysis — append update without full rewrite"
        >
          Update Project
        </button>
        <button onClick={onDelete} disabled={busy}>
          Delete
        </button>
        <button onClick={onExportMd} disabled={busy || !a}>
          Export Markdown
        </button>
        <button onClick={() => void onExportBundle()} disabled={busy || !a}>
          Export Project
        </button>
      </div>

      {updateMsg && <p className="muted">{updateMsg}</p>}
      {insightError && (
        <div className="error-banner" style={{ marginTop: "0.75rem" }}>
          {insightError}
        </div>
      )}

      {importance?.top_insights?.length ? (
        <section className="card" style={{ marginTop: "0.75rem" }}>
          <h3 style={{ marginTop: 0 }}>Top 3 insights</h3>
          <ul className="op-list">
            {importance.top_insights.map((x, i) => (
              <li key={i}>{x}</li>
            ))}
          </ul>
        </section>
      ) : null}

      <section className="card" style={{ marginTop: "0.75rem" }}>
        <h3 style={{ marginTop: 0 }}>LinkedIn post generator</h3>
        <div style={{ display: "flex", gap: "0.5rem", flexWrap: "wrap" }}>
          <select value={linkedinLength} onChange={(e) => setLinkedinLength(e.target.value as "short" | "long")}>
            <option value="short">Short (2-3 lines)</option>
            <option value="long">Long (5-8 lines)</option>
          </select>
          <select
            value={linkedinFocus}
            onChange={(e) =>
              setLinkedinFocus(
                e.target.value as
                  | "describe_tool"
                  | "describe_stack"
                  | "describe_problem"
                  | "describe_outcome"
              )
            }
          >
            <option value="describe_tool">Describe tool</option>
            <option value="describe_stack">Describe stack</option>
            <option value="describe_problem">Describe problem</option>
            <option value="describe_outcome">Describe outcome</option>
          </select>
          <button type="button" className="btn btn-primary" disabled={busy || !a} onClick={() => void onGenerateLinkedin()}>
            Generate
          </button>
        </div>
        {linkedinText ? <pre className="proof-body" style={{ marginTop: "0.75rem" }}>{linkedinText}</pre> : null}
      </section>

      {a && (
        <section className="living-panel card" style={{ marginTop: "1rem" }}>
          <h3 className="living-panel-title">Living system</h3>
          <p className="meta" style={{ marginTop: 0 }}>
            Positioning anchor and next upgrades use your stored analysis. Record
            update rescans the folder and appends changes to the timeline below.
          </p>
          <div className="living-actions">
            <button
              type="button"
              className="btn btn-primary"
              disabled={busy}
              onClick={() => void onLoadPositioning()}
            >
              Positioning clarity
            </button>
            <button
              type="button"
              className="btn"
              disabled={busy}
              onClick={() => void onLoadEvolutionSuggestions()}
            >
              Next upgrades (2–3)
            </button>
          </div>

          {positioning && (
            <div className="living-card">
              <h4>Positioning anchor</h4>
              <p>
                <strong>Category:</strong> {positioning.category}
              </p>
              <p>
                <strong>Primary audience:</strong> {positioning.primary_audience}
              </p>
              <p className="positioning-anchor">{positioning.one_sentence_anchor}</p>
            </div>
          )}

          {evolutionIdeas && (
            <div className="living-card">
              <h4>Suggested next steps</h4>
              <ul className="evolution-suggest-list">
                {evolutionIdeas.suggestions.map((s, i) => (
                  <li key={i}>
                    <strong>{s.title}</strong>
                    <p className="meta">{s.why}</p>
                    <p className="meta">{s.build_notes}</p>
                  </li>
                ))}
              </ul>
            </div>
          )}
        </section>
      )}

      {(project.evolutions ?? []).length > 0 && (
        <section className="detail-section">
          <h3>Update timeline</h3>
          <p className="meta">
            Incremental scans — new features and changes appended over time (full
            analysis unchanged).
          </p>
          <ul className="evolution-timeline">
            {(project.evolutions ?? []).map((ev) => (
              <li key={ev.id} className="card evolution-entry">
                <strong>{ev.label}</strong>
                <span className="meta"> · {new Date(ev.created_at).toLocaleString()}</span>
                <ul className="op-list">
                  {ev.new_features.map((f, i) => (
                    <li key={i}>{f}</li>
                  ))}
                </ul>
                <p className="op-body" style={{ marginTop: "0.5rem" }}>
                  {ev.summary}
                </p>
              </li>
            ))}
          </ul>
        </section>
      )}

      {!a && (
        <p className="muted">
          No analysis yet. Click Re-analyze.
        </p>
      )}

      {a && (
        <>
          <div style={{ display: "flex", gap: "0.5rem", flexWrap: "wrap", marginTop: "0.75rem" }}>
            <button
              className="btn"
              type="button"
              onClick={() => setCollapsed((c) => ({ ...c, intelligence: !c.intelligence }))}
            >
              {collapsed.intelligence ? "Show product intelligence" : "Hide product intelligence"}
            </button>
            <button
              className="btn"
              type="button"
              onClick={() => setCollapsed((c) => ({ ...c, details: !c.details }))}
            >
              {collapsed.details ? "Show analysis details" : "Hide analysis details"}
            </button>
          </div>
          {/* 🔥 PRODUCT INTELLIGENCE */}
          {!collapsed.intelligence && a.product_intelligence && (
            <div className="product-intelligence-card">
              <h3>Product Intelligence</h3>

              <p><strong>Category:</strong> {a.product_intelligence.category}</p>
              <p><strong>Stage:</strong> {a.product_intelligence.product_stage}</p>

              <h4>Target Users</h4>
              <ul>
                {a.product_intelligence.target_users?.slice(0, 3).map((x, i) => (
                  <li key={i}>{x}</li>
                ))}
              </ul>

              <h4>Use Cases</h4>
              <ul>
                {a.product_intelligence.use_cases?.slice(0, 3).map((x, i) => (
                  <li key={i}>{x}</li>
                ))}
              </ul>

              <h4>Monetization</h4>
              <ul>
                {a.product_intelligence.monetization_models?.slice(0, 3).map((x, i) => (
                  <li key={i}>{x}</li>
                ))}
              </ul>

              <h4>Strengths</h4>
              <ul>
                {a.product_intelligence.strengths?.slice(0, 3).map((x, i) => (
                  <li key={i}>{x}</li>
                ))}
              </ul>

              <h4>Risks</h4>
              <ul>
                {a.product_intelligence.risks?.slice(0, 3).map((x, i) => (
                  <li key={i}>{x}</li>
                ))}
              </ul>

              <h4>What's Missing</h4>
              <ul>
                {a.product_intelligence.what_is_missing?.slice(0, 3).map((x, i) => (
                  <li key={i}>{x}</li>
                ))}
              </ul>

              {/* Go-To-Market */}
              {a.product_intelligence.go_to_market && (
                <>
                  <h4>Go To Market</h4>
                  <p><strong>Target:</strong> {a.product_intelligence.go_to_market.target_user}</p>
                  <p><strong>Sell as:</strong> {a.product_intelligence.go_to_market.sell_as}</p>

                  <h5>Where to sell</h5>
                  <ul>
                    {a.product_intelligence.go_to_market.where_to_sell?.slice(0, 3).map((x, i) => (
                      <li key={i}>{x}</li>
                    ))}
                  </ul>

                  <h5>First steps</h5>
                  <ul>
                    {a.product_intelligence.go_to_market.first_steps?.slice(0, 3).map((x, i) => (
                      <li key={i}>{x}</li>
                    ))}
                  </ul>
                </>
              )}
            </div>
          )}

          {/* NORMAL SECTIONS */}
          {!collapsed.details && (
          <>
          <section>
            <h3>Summary</h3>
            <p>{a.one_line_summary}</p>
          </section>

          <section>
            <h3>Intent</h3>
            <p>{a.project_intent}</p>
          </section>

          <section>
            <h3>Why it matters</h3>
            <p>{a.why_it_matters}</p>
          </section>

          <section>
            <h3>Deep explanation</h3>
            <p>{a.deep_explanation}</p>
          </section>

          {a.full_narrative_explanation && (
            <section>
              <h3>Full Narrative</h3>
              <p style={{ whiteSpace: "pre-wrap" }}>{narrativePreview}</p>
              {a.full_narrative_explanation.length > NARRATIVE_PREVIEW_CHARS ? (
                <button
                  type="button"
                  className="btn"
                  onClick={() => setNarrativeExpanded((v) => !v)}
                >
                  {narrativeExpanded ? "Show less" : "Show full narrative"}
                </button>
              ) : null}
            </section>
          )}
          </>
          )}
        </>
      )}
    </div>
  );
}