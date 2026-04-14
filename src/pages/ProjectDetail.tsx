import { useCallback, useEffect, useState } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";
import { invoke } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";
import type {
  EvolutionSuggestionsPayload,
  IncrementalUpdateResult,
  PositioningPayload,
  ProjectDetail as PD,
} from "../types";

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

  const load = useCallback(async () => {
    if (!id) return;
    setError(null);
    try {
      const p = await invoke<PD | null>("get_project", {
        id: Number(id),
      });
      setProject(p);
    } catch (e) {
      setError(String(e));
    }
  }, [id]);

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => {
    setPositioning(null);
    setEvolutionIdeas(null);
    setInsightError(null);
    setUpdateMsg(null);
  }, [id]);

  async function onReanalyze() {
    if (!id) return;
    setBusy(true);
    setError(null);
    try {
      await invoke("reanalyze_project", { id: Number(id) });
      await load();
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
      await load();
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
          Record update
        </button>
        <button onClick={onDelete} disabled={busy}>
          Delete
        </button>
        <button onClick={onExportMd} disabled={busy || !a}>
          Export Markdown
        </button>
      </div>

      {updateMsg && <p className="muted">{updateMsg}</p>}
      {insightError && (
        <div className="error-banner" style={{ marginTop: "0.75rem" }}>
          {insightError}
        </div>
      )}

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
          {/* 🔥 PRODUCT INTELLIGENCE */}
          {a.product_intelligence && (
            <div className="product-intelligence-card">
              <h3>Product Intelligence</h3>

              <p><strong>Category:</strong> {a.product_intelligence.category}</p>
              <p><strong>Stage:</strong> {a.product_intelligence.product_stage}</p>

              <h4>Target Users</h4>
              <ul>
                {a.product_intelligence.target_users?.map((x, i) => (
                  <li key={i}>{x}</li>
                ))}
              </ul>

              <h4>Use Cases</h4>
              <ul>
                {a.product_intelligence.use_cases?.map((x, i) => (
                  <li key={i}>{x}</li>
                ))}
              </ul>

              <h4>Monetization</h4>
              <ul>
                {a.product_intelligence.monetization_models?.map((x, i) => (
                  <li key={i}>{x}</li>
                ))}
              </ul>

              <h4>Strengths</h4>
              <ul>
                {a.product_intelligence.strengths?.map((x, i) => (
                  <li key={i}>{x}</li>
                ))}
              </ul>

              <h4>Risks</h4>
              <ul>
                {a.product_intelligence.risks?.map((x, i) => (
                  <li key={i}>{x}</li>
                ))}
              </ul>

              <h4>What's Missing</h4>
              <ul>
                {a.product_intelligence.what_is_missing?.map((x, i) => (
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
                    {a.product_intelligence.go_to_market.where_to_sell?.map((x, i) => (
                      <li key={i}>{x}</li>
                    ))}
                  </ul>

                  <h5>First steps</h5>
                  <ul>
                    {a.product_intelligence.go_to_market.first_steps?.map((x, i) => (
                      <li key={i}>{x}</li>
                    ))}
                  </ul>
                </>
              )}
            </div>
          )}

          {/* NORMAL SECTIONS */}
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
              <p style={{ whiteSpace: "pre-wrap" }}>
                {a.full_narrative_explanation}
              </p>
            </section>
          )}
        </>
      )}
    </div>
  );
}