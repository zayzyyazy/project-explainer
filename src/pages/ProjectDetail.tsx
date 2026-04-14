import { useCallback, useEffect, useState } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";
import { invoke } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";
import type { ProjectDetail as PD } from "../types";

export default function ProjectDetail() {
  const { id } = useParams();
  const navigate = useNavigate();

  const [project, setProject] = useState<PD | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

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
        <button onClick={onDelete} disabled={busy}>
          Delete
        </button>
        <button onClick={onExportMd} disabled={busy || !a}>
          Export Markdown
        </button>
      </div>

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