import { useCallback, useEffect, useMemo, useState } from "react";
import { Link } from "react-router-dom";
import { invoke } from "@tauri-apps/api/core";
import type { OpportunityPayload, ProjectListItem } from "../types";

export default function Opportunities() {
  const [projects, setProjects] = useState<ProjectListItem[]>([]);
  const [projectId, setProjectId] = useState<number | "">("");
  const [payload, setPayload] = useState<OpportunityPayload | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const loadProjects = useCallback(async () => {
    setError(null);
    try {
      const list = await invoke<ProjectListItem[]>("list_projects");
      setProjects(list);
    } catch (e) {
      setError(String(e));
    }
  }, []);

  useEffect(() => {
    void loadProjects();
  }, [loadProjects]);

  const analyzedIds = useMemo(() => {
    return new Set(
      projects
        .filter((p) => p.last_analyzed_at != null)
        .map((p) => p.id),
    );
  }, [projects]);

  async function onGenerate() {
    if (projectId === "") return;
    setBusy(true);
    setError(null);
    setPayload(null);
    try {
      const result = await invoke<OpportunityPayload>("generate_opportunities", {
        id: projectId,
      });
      setPayload(result);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div>
      <p className="muted" style={{ marginTop: 0, marginBottom: "1rem" }}>
        Turn an existing project analysis into 3–5 narrow, sellable opportunities. Nothing is
        saved to your library—generation is on demand only.
      </p>

      {error && <div className="error-banner">{error}</div>}

      <div
        className="opportunities-toolbar"
        style={{
          display: "flex",
          flexWrap: "wrap",
          gap: "0.75rem",
          alignItems: "center",
          marginBottom: "1.25rem",
        }}
      >
        <label className="muted" style={{ display: "flex", alignItems: "center", gap: "0.5rem" }}>
          Project
          <select
            className="search"
            style={{ minWidth: "220px" }}
            value={projectId === "" ? "" : String(projectId)}
            onChange={(e) => {
              const v = e.target.value;
              setProjectId(v === "" ? "" : Number(v));
              setPayload(null);
              setError(null);
            }}
            aria-label="Select analyzed project"
          >
            <option value="">Choose a project…</option>
            {projects.map((p) => (
              <option key={p.id} value={p.id} disabled={!analyzedIds.has(p.id)}>
                {p.name}
                {!analyzedIds.has(p.id) ? " (not analyzed)" : ""}
              </option>
            ))}
          </select>
        </label>
        <button
          type="button"
          className="btn btn-primary"
          disabled={busy || projectId === "" || !analyzedIds.has(Number(projectId))}
          onClick={() => void onGenerate()}
        >
          {busy ? "Generating…" : "Generate opportunities"}
        </button>
        <Link to="/" className="muted">
          ← Back to projects
        </Link>
      </div>

      {payload && (
        <div className="opportunity-grid">
          {payload.opportunities.map((o, idx) => (
            <article key={`${idx}-${o.title}`} className="card opportunity-card">
              <h2>{o.title}</h2>
              <OpRow label="What it is" text={o.what_it_is} />
              <OpRow label="Problem" text={o.problem} />
              <OpRow label="Why now" text={o.why_this_problem_is_real_now} />
              <OpRow label="Target customer" text={o.target_customer} />
              <OpRow label="Who to contact" text={o.who_exactly_to_contact} />
              <OpRow label="Packaging" text={o.how_to_package} />
              <OpRow label="Pricing logic" text={o.pricing_logic} />
              <OpList label="Distribution" items={o.distribution_strategy} />
              <OpList label="First 3 validation steps" items={o.first_3_steps_to_validate} />
              <OpRow label="Risk level" text={o.risk_level} />
              <OpRow label="Why it could fail" text={o.why_this_could_fail} />
            </article>
          ))}
        </div>
      )}
    </div>
  );
}

function OpRow({ label, text }: { label: string; text: string }) {
  return (
    <div className="op-field">
      <div className="op-label">{label}</div>
      <p className="op-body">{text}</p>
    </div>
  );
}

function OpList({ label, items }: { label: string; items: string[] }) {
  return (
    <div className="op-field">
      <div className="op-label">{label}</div>
      <ul className="op-list">
        {items.map((x) => (
          <li key={x}>{x}</li>
        ))}
      </ul>
    </div>
  );
}
