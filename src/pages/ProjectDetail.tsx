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
      navigate("/");
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

  if (!project && !error) {
    return <p className="muted">Loading…</p>;
  }

  if (!project) {
    return (
      <div>
        {error && <div className="error-banner">{error}</div>}
        <p>Project not found.</p>
        <Link to="/">← Back</Link>
      </div>
    );
  }

  const a = project.analysis;

  return (
    <div>
      {error && <div className="error-banner">{error}</div>}

      <p>
        <Link to="/">← Dashboard</Link>
      </p>

      <h2>{a?.project_name ?? project.name}</h2>

      <p className="meta" title={project.path}>
        {project.path}
      </p>

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

      <p className="meta">
        Last analyzed:{" "}
        {project.last_analyzed_at
          ? new Date(project.last_analyzed_at).toLocaleString()
          : "—"}
      </p>

      {!a && (
        <p className="muted">
          No analysis yet. Click Re-analyze.
        </p>
      )}

      {a && !a.product_intelligence && (
        <p className="muted">
          Re-analyze to generate the Product Intelligence section for this project.
        </p>
      )}

      {a && (
        <>
          {a.product_intelligence && (
            <div className="product-intelligence-card">
              <h3 className="product-intelligence-title">Product Intelligence</h3>
              <p className="muted product-intelligence-sub">
                What you built and how it could become something sellable—grounded in this repo, not generic advice.
              </p>
              <div className="pi-row">
                <span className="pi-label">Category</span>
                <span className="pi-category-tag">
                  {a.product_intelligence.category}
                </span>
              </div>
              <div className="pi-row">
                <span className="pi-label">Product stage</span>
                <span className="pi-stage-badge">
                  {a.product_intelligence.product_stage}
                </span>
              </div>
              <div className="pi-block">
                <h4>Target users</h4>
                <ul>
                  {a.product_intelligence.target_users.map((x, i) => (
                    <li key={`pi-tu-${i}`}>{x}</li>
                  ))}
                </ul>
              </div>
              <div className="pi-block">
                <h4>Use cases</h4>
                <ul>
                  {a.product_intelligence.use_cases.map((x, i) => (
                    <li key={`pi-uc-${i}`}>{x}</li>
                  ))}
                </ul>
              </div>
              <div className="pi-block">
                <h4>Monetization models</h4>
                <ul>
                  {a.product_intelligence.monetization_models.map((x, i) => (
                    <li key={`pi-mm-${i}`}>{x}</li>
                  ))}
                </ul>
              </div>
              <div className="pi-block">
                <h4>Distribution channels</h4>
                <ul>
                  {a.product_intelligence.distribution_channels.map((x, i) => (
                    <li key={`pi-dc-${i}`}>{x}</li>
                  ))}
                </ul>
              </div>
              <div className="pi-block">
                <h4>Strengths</h4>
                <ul>
                  {a.product_intelligence.strengths.map((x, i) => (
                    <li key={`pi-st-${i}`}>{x}</li>
                  ))}
                </ul>
              </div>
              <div className="pi-block">
                <h4>Risks</h4>
                <ul>
                  {a.product_intelligence.risks.map((x, i) => (
                    <li key={`pi-rk-${i}`}>{x}</li>
                  ))}
                </ul>
              </div>
              <div className="pi-block pi-missing">
                <h4>What&apos;s missing</h4>
                <ul>
                  {a.product_intelligence.what_is_missing.map((x, i) => (
                    <li key={`pi-wm-${i}`}>{x}</li>
                  ))}
                </ul>
              </div>
            </div>
          )}

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

          {a.full_narrative_explanation ? (
            <section className="narrative-section">
              <h3>Full Narrative Explanation</h3>
              <div className="narrative-block">
                {a.full_narrative_explanation}
              </div>
            </section>
          ) : null}

          <section>
            <h3>Core features</h3>
            <ul>
              {a.core_features.map((f) => (
                <li key={f}>{f}</li>
              ))}
            </ul>
          </section>

          <section>
            <h3>Key flows</h3>
            <ul>
              {a.key_flows.map((f) => (
                <li key={f}>{f}</li>
              ))}
            </ul>
          </section>

          <section>
            <h3>How it works</h3>
            <ul>
              {a.how_it_works_step_by_step.map((s) => (
                <li key={s}>{s}</li>
              ))}
            </ul>
          </section>

          <section>
            <h3>Design decisions</h3>
            <ul>
              {a.design_decisions.map((d) => (
                <li key={d}>{d}</li>
              ))}
            </ul>
          </section>

          <section>
            <h3>Limitations</h3>
            <ul>
              {a.tradeoffs_and_limitations.map((l) => (
                <li key={l}>{l}</li>
              ))}
            </ul>
          </section>

          <section>
            <h3>How to run</h3>
            <p style={{ whiteSpace: "pre-wrap" }}>{a.how_to_run}</p>
          </section>

          <section>
            <h3>Important files</h3>
            <ul>
              {a.important_files.map((f) => (
                <li key={f.path}>
                  <strong>{f.path}</strong> — {f.why_it_matters}
                </li>
              ))}
            </ul>
          </section>
        </>
      )}

      {project.file_index_sample.length > 0 && (
        <section>
          <h3>Indexed files (sample)</h3>
          <pre>
            {project.file_index_sample.join("\n")}
            {project.raw_file_list_truncated ? "\n…" : ""}
          </pre>
        </section>
      )}
    </div>
  );
}