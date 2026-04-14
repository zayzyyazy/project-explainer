import { useCallback, useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { invoke } from "@tauri-apps/api/core";
import type { IdeaProject } from "../types";

export default function IdeaProjects() {
  const [items, setItems] = useState<IdeaProject[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [expandedId, setExpandedId] = useState<number | null>(null);
  const [deletingId, setDeletingId] = useState<number | null>(null);

  const load = useCallback(async () => {
    setError(null);
    try {
      const list = await invoke<IdeaProject[]>("list_idea_projects");
      setItems(list);
    } catch (e) {
      setError(String(e));
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  async function onDelete(id: number) {
    if (!window.confirm("Remove this saved idea from your library?")) return;
    setDeletingId(id);
    setError(null);
    try {
      await invoke("delete_idea_project", { id });
      if (expandedId === id) setExpandedId(null);
      await load();
    } catch (e) {
      setError(String(e));
    } finally {
      setDeletingId(null);
    }
  }

  return (
    <div>
      <p>
        <Link to="/dashboard">← Dashboard</Link>
        {" · "}
        <Link to="/opportunities">Opportunities</Link>
      </p>

      <h2>Idea Projects</h2>
      <p className="meta">
        Saved sellable ideas from the Opportunities dashboard. They persist after you quit
        the app.
      </p>

      {error && <div className="error-banner">{error}</div>}

      {items.length === 0 && !error ? (
        <p className="muted">
          No saved ideas yet. Generate opportunities and click &quot;Save Idea&quot; on a card.
        </p>
      ) : (
        <div className="idea-projects-grid">
          {items.map((idea) => {
            const open = expandedId === idea.id;
            return (
              <article key={idea.id} className="card idea-project-card">
                <div className="idea-project-head">
                  <button
                    type="button"
                    className="idea-project-title-btn"
                    onClick={() =>
                      setExpandedId(open ? null : idea.id)
                    }
                    aria-expanded={open}
                  >
                    <h3>{idea.title}</h3>
                  </button>
                  <div className="idea-project-actions">
                    <button
                      type="button"
                      className="btn btn-danger"
                      disabled={deletingId === idea.id}
                      onClick={() => void onDelete(idea.id)}
                    >
                      {deletingId === idea.id ? "Removing…" : "Remove"}
                    </button>
                  </div>
                </div>
                <p className="meta" style={{ marginTop: 0 }}>
                  From{" "}
                  <Link to={`/project/${idea.source_project_id}`}>
                    {idea.source_project_name}
                  </Link>
                  {" · "}
                  Saved {new Date(idea.saved_at).toLocaleString()}
                </p>
                {!open ? (
                  <p className="op-body" style={{ marginTop: "0.5rem" }}>
                    {idea.what_it_is.length > 180
                      ? `${idea.what_it_is.slice(0, 180)}…`
                      : idea.what_it_is}
                  </p>
                ) : (
                  <div className="idea-project-detail">
                    <Field label="What it is" text={idea.what_it_is} />
                    <Field label="Problem" text={idea.problem} />
                    <Field label="Why now" text={idea.why_this_problem_is_real_now} />
                    <Field label="Target customer" text={idea.target_customer} />
                    <Field label="Who to contact" text={idea.who_exactly_to_contact} />
                    <Field label="Packaging" text={idea.how_to_package} />
                    <Field label="Pricing logic" text={idea.pricing_logic} />
                    <ListField label="Distribution" items={idea.distribution_strategy} />
                    <ListField
                      label="First 3 validation steps"
                      items={idea.first_3_steps_to_validate}
                    />
                    <Field label="Risk level" text={idea.risk_level} />
                    <Field label="Why it could fail" text={idea.why_this_could_fail} />
                  </div>
                )}
              </article>
            );
          })}
        </div>
      )}
    </div>
  );
}

function Field({ label, text }: { label: string; text: string }) {
  return (
    <div className="op-field">
      <span className="op-label">{label}</span>
      <p className="op-body">{text}</p>
    </div>
  );
}

function ListField({ label, items }: { label: string; items: string[] }) {
  return (
    <div className="op-field">
      <span className="op-label">{label}</span>
      <ul className="op-list">
        {items.map((x, idx) => (
          <li key={idx}>{x}</li>
        ))}
      </ul>
    </div>
  );
}
