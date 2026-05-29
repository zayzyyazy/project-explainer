import { useCallback, useEffect, useMemo, useState } from "react";
import { Link } from "react-router-dom";
import { invoke } from "@tauri-apps/api/core";
import type {
  AiOpportunitiesResult,
  Opportunity,
  OpportunityPayload,
  ProjectListItem,
} from "../types";

export default function Opportunities() {
  const [projects, setProjects] = useState<ProjectListItem[]>([]);
  const [selectedId, setSelectedId] = useState<string>("");
  const [data, setData] = useState<OpportunityPayload | null>(null);
  const [fromCache, setFromCache] = useState<boolean | null>(null);
  const [oppBusy, setOppBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [savingKey, setSavingKey] = useState<string | null>(null);
  const [savedKeys, setSavedKeys] = useState<Set<string>>(() => new Set());

  const loadProjects = useCallback(async () => {
    setError(null);
    try {
      const rows = await invoke<ProjectListItem[]>("list_projects");
      setProjects(rows);
    } catch (e) {
      setError(String(e));
    }
  }, []);

  useEffect(() => {
    void loadProjects();
  }, [loadProjects]);

  useEffect(() => {
    const on = () => void loadProjects();
    window.addEventListener("peo:projects-changed", on);
    return () => window.removeEventListener("peo:projects-changed", on);
  }, [loadProjects]);

  const analyzedProjects = useMemo(
    () => projects.filter((p) => !!p.last_analyzed_at),
    [projects]
  );

  const fetchOpportunities = useCallback(
    async (regenerate: boolean) => {
      if (!selectedId) return;

      setOppBusy(true);
      setError(null);
      setSaveError(null);

      if (!regenerate) {
        setData(null);
        setFromCache(null);
      }

      await new Promise<void>((resolve) => setTimeout(resolve, 0));

      try {
        const result = await invoke<AiOpportunitiesResult>(
          "generate_opportunities",
          {
            args: {
              id: Number(selectedId),
              regenerate,
            },
          }
        );

        setData(result.payload);
        setFromCache(result.from_cache);
      } catch (e) {
        setError(String(e));
      } finally {
        setOppBusy(false);
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

  const selectedProjectName = useMemo(() => {
    if (!selectedId) return "";
    return analyzedProjects.find((p) => p.id === Number(selectedId))?.name ?? "";
  }, [analyzedProjects, selectedId]);

  function ideaKey(op: Opportunity) {
    return `${selectedId}::${op.title}`;
  }

  async function onSaveIdea(op: Opportunity) {
    if (!selectedId) return;

    const key = ideaKey(op);
    setSaveError(null);
    setSavingKey(key);

    try {
      await invoke<number>("save_idea_project", {
        input: {
          source_project_id: Number(selectedId),
          title: op.title,
          what_it_is: op.what_it_is,
          problem: op.problem,
          why_this_problem_is_real_now: op.why_this_problem_is_real_now,
          target_customer: op.target_customer,
          who_exactly_to_contact: op.who_exactly_to_contact,
          how_to_package: op.how_to_package,
          pricing_logic: op.pricing_logic,
          distribution_strategy: op.distribution_strategy,
          first_3_steps_to_validate: op.first_3_steps_to_validate,
          risk_level: op.risk_level,
          why_this_could_fail: op.why_this_could_fail,
        },
      });

      setSavedKeys((prev) => {
        const next = new Set(prev);
        next.add(key);
        return next;
      });
    } catch (e) {
      setSaveError(String(e));
    } finally {
      setSavingKey(null);
    }
  }

  return (
    <div>
      <p>
        <Link to="/">← Dashboard</Link>
      </p>

      <h2>Opportunities Dashboard</h2>
      <p className="meta">
        Generate realistic sellable ideas from projects you already built.
      </p>

      {error && <div className="error-banner">{error}</div>}
      {saveError && <div className="error-banner">{saveError}</div>}

      <section className="detail-section">
        <h3>Select project</h3>

        {analyzedProjects.length === 0 ? (
          <p className="muted">
            No analyzed projects found yet. Analyze a project first.
          </p>
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

            <div
              style={{
                marginTop: "1rem",
                display: "flex",
                gap: "0.75rem",
                flexWrap: "wrap",
              }}
            >
              <button
                type="button"
                className="btn btn-primary"
                onClick={() => void fetchOpportunities(false)}
                disabled={oppBusy || !selectedId}
              >
                {oppBusy && !data ? "Loading…" : "Load opportunities"}
              </button>

              <button
                type="button"
                onClick={() => void fetchOpportunities(true)}
                disabled={oppBusy || !selectedId}
              >
                {oppBusy && !!data ? "Working…" : "Regenerate"}
              </button>

              {fromCache === true && !oppBusy && selectedId ? (
                <span
                  className="muted"
                  style={{ alignSelf: "center", fontSize: "0.85rem" }}
                >
                  Loaded from saved results
                </span>
              ) : null}
            </div>

            {oppBusy && selectedId ? (
              <p className="muted" style={{ marginTop: "0.75rem" }}>
                {data ? "Regenerating opportunities…" : "Loading opportunities…"}
              </p>
            ) : null}
          </>
        )}
      </section>

      {data?.opportunities?.length ? (
        <div className="opportunity-grid">
          {data.opportunities.map((op, i) => (
            <article key={`${op.title}-${i}`} className="opportunity-card">
              <div className="opportunity-card-head">
                <h3>{op.title}</h3>

                <div className="opportunity-save-row">
                  <button
                    type="button"
                    className="btn btn-primary"
                    disabled={
                      oppBusy ||
                      !selectedId ||
                      savingKey === ideaKey(op) ||
                      savedKeys.has(ideaKey(op))
                    }
                    onClick={() => void onSaveIdea(op)}
                  >
                    {savingKey === ideaKey(op)
                      ? "Saving…"
                      : savedKeys.has(ideaKey(op))
                        ? "Saved"
                        : "Save Idea"}
                  </button>

                  {selectedProjectName ? (
                    <span className="muted" style={{ fontSize: "0.8rem" }}>
                      From: {selectedProjectName}
                    </span>
                  ) : null}
                </div>
              </div>

              <div className="op-field">
                <span className="op-label">What it is</span>
                <p className="op-body">{op.what_it_is}</p>
              </div>

              <div className="op-field">
                <span className="op-label">Problem</span>
                <p className="op-body">{op.problem}</p>
              </div>

              <div className="op-field">
                <span className="op-label">Why now</span>
                <p className="op-body">{op.why_this_problem_is_real_now}</p>
              </div>

              <div className="op-field">
                <span className="op-label">Target customer</span>
                <p className="op-body">{op.target_customer}</p>
              </div>

              <div className="op-field">
                <span className="op-label">Who to contact</span>
                <p className="op-body">{op.who_exactly_to_contact}</p>
              </div>

              <div className="op-field">
                <span className="op-label">How to package</span>
                <p className="op-body">{op.how_to_package}</p>
              </div>

              <div className="op-field">
                <span className="op-label">Pricing logic</span>
                <p className="op-body">{op.pricing_logic}</p>
              </div>

              <div className="op-field">
                <span className="op-label">Distribution strategy</span>
                <ul className="op-list">
                  {op.distribution_strategy.map((x, idx) => (
                    <li key={idx}>{x}</li>
                  ))}
                </ul>
              </div>

              <div className="op-field">
                <span className="op-label">First 3 steps to validate</span>
                <ul className="op-list">
                  {op.first_3_steps_to_validate.map((x, idx) => (
                    <li key={idx}>{x}</li>
                  ))}
                </ul>
              </div>

              <div className="op-field">
                <span className="op-label">Risk level</span>
                <p className="op-body">{op.risk_level}</p>
              </div>

              <div className="op-field">
                <span className="op-label">Why this could fail</span>
                <p className="op-body">{op.why_this_could_fail}</p>
              </div>
            </article>
          ))}
        </div>
      ) : null}
    </div>
  );
}