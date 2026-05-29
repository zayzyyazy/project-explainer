import { useCallback, useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { invoke } from "@tauri-apps/api/core";
import type { UserProfile } from "../types";

const ROLES: { value: string; label: string }[] = [
  { value: "freelancer", label: "Freelancer" },
  { value: "indie_hacker", label: "Indie hacker" },
  { value: "developer", label: "Developer" },
];

const BUILD: { value: string; label: string }[] = [
  { value: "client_work", label: "Client work" },
  { value: "saas", label: "SaaS / products" },
  { value: "internal_tools", label: "Internal tools" },
];

const GOALS: { value: string; label: string }[] = [
  { value: "get_clients", label: "Win clients / close work" },
  { value: "portfolio", label: "Portfolio & proposals" },
  { value: "proposals", label: "Proposals & pitches" },
  { value: "archive", label: "Archive & clarity" },
];

export default function Setup() {
  const [role, setRole] = useState<string>("");
  const [build, setBuild] = useState<Set<string>>(new Set());
  const [goal, setGoal] = useState<string>("");
  const [error, setError] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);
  const [loading, setLoading] = useState(true);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const p = await invoke<UserProfile | null>("get_user_profile");
      if (p) {
        setRole(p.role ?? "");
        setBuild(new Set(p.what_i_build ?? []));
        setGoal(p.app_goal ?? "");
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  function toggleBuild(v: string) {
    setBuild((prev) => {
      const n = new Set(prev);
      if (n.has(v)) n.delete(v);
      else n.add(v);
      return n;
    });
  }

  async function onSave() {
    setError(null);
    setSaved(false);
    const profile: UserProfile = {
      role: role || null,
      what_i_build: Array.from(build),
      app_goal: goal || null,
    };
    try {
      await invoke("save_user_profile", { profile });
      setSaved(true);
    } catch (e) {
      setError(String(e));
    }
  }

  return (
    <div>
      <p>
        <Link to="/dashboard">← Dashboard</Link>
      </p>

      <h2>Writer profile</h2>
      <p className="meta">
        Optional. When set, case studies and positioning shift tone: freelancer →
        client value; indie → product; developer → technical depth. Leave blank for
        a balanced default.
      </p>

      {loading && <p className="muted">Loading…</p>}
      {error && <div className="error-banner">{error}</div>}
      {saved && <p className="muted">Saved.</p>}

      {!loading && (
        <div className="card setup-form" style={{ marginTop: "1rem", maxWidth: 480 }}>
          <div className="op-field">
            <span className="op-label">What I am</span>
            <select
              className="search"
              style={{ width: "100%", maxWidth: "100%" }}
              value={role}
              onChange={(e) => setRole(e.target.value)}
            >
              <option value="">— Not set —</option>
              {ROLES.map((r) => (
                <option key={r.value} value={r.value}>
                  {r.label}
                </option>
              ))}
            </select>
          </div>

          <div className="op-field">
            <span className="op-label">What I build (any)</span>
            <div className="setup-checks">
              {BUILD.map((b) => (
                <label key={b.value} className="setup-check">
                  <input
                    type="checkbox"
                    checked={build.has(b.value)}
                    onChange={() => toggleBuild(b.value)}
                  />{" "}
                  {b.label}
                </label>
              ))}
            </div>
          </div>

          <div className="op-field">
            <span className="op-label">What I want from the app</span>
            <select
              className="search"
              style={{ width: "100%", maxWidth: "100%" }}
              value={goal}
              onChange={(e) => setGoal(e.target.value)}
            >
              <option value="">— Not set —</option>
              {GOALS.map((g) => (
                <option key={g.value} value={g.value}>
                  {g.label}
                </option>
              ))}
            </select>
          </div>

          <button type="button" className="btn btn-primary" onClick={() => void onSave()}>
            Save profile
          </button>
        </div>
      )}

      <p className="muted" style={{ marginTop: "1.5rem" }}>
        Stored only on this machine.{" "}
        <Link to="/case-study">Case Study</Link>
      </p>
    </div>
  );
}
