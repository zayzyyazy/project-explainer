import { useEffect, useState } from "react";
import { Link, Outlet } from "react-router-dom";
import { invoke } from "@tauri-apps/api/core";
import type { UserProfile } from "../types";
import { isUserProfileFilled } from "../types";

function formatProfileLine(p: UserProfile | null): string {
  if (!isUserProfileFilled(p)) return "";
  const parts: string[] = [];
  if (p?.role) parts.push(p.role.replace(/_/g, " "));
  if (p?.what_i_build?.length)
    parts.push(p.what_i_build.map((x) => x.replace(/_/g, " ")).join(", "));
  if (p?.app_goal) parts.push(`Goal: ${p.app_goal.replace(/_/g, " ")}`);
  return parts.join(" · ");
}

export default function AppLayout() {
  const [profile, setProfile] = useState<UserProfile | null>(null);

  useEffect(() => {
    void (async () => {
      try {
        const p = await invoke<UserProfile | null>("get_user_profile");
        setProfile(p);
      } catch {
        setProfile(null);
      }
    })();
  }, []);

  const line = formatProfileLine(profile);

  return (
    <div className="app-shell">
      <header className="app-header-block">
        <div className="top-bar">
          <Link
            to="/dashboard"
            style={{ textDecoration: "none", color: "inherit" }}
          >
            <h1>Project Explainer OS</h1>
          </Link>

          <nav className="top-nav">
            <Link to="/dashboard">Projects</Link>
            <Link to="/settings">Settings</Link>
            <Link to="/setup">Profile</Link>
            <Link to="/opportunities">Opportunities</Link>
            <Link to="/case-study">Case study</Link>
            <Link to="/idea-projects">Ideas</Link>
            <a href="https://example.com/feedback" target="_blank" rel="noreferrer">
              Feedback
            </a>
          </nav>
        </div>

        <div className="profile-strip">
          {line ? (
            <p className="profile-strip-text">
              <span className="profile-strip-label">Builder lens</span> {line}{" "}
              <Link to="/setup" className="profile-strip-edit">
                Edit
              </Link>
            </p>
          ) : (
            <p className="profile-strip-text muted">
              Set your{" "}
              <Link to="/setup">writer profile</Link> so outputs match how you
              work and what you want.
            </p>
          )}
        </div>
      </header>

      <main>
        <Outlet />
      </main>
      <footer className="meta" style={{ marginTop: "1rem", opacity: 0.9 }}>
        v0.9.0 beta · Data stays local. Only sent to Claude/OpenAI API for generation.
      </footer>
    </div>
  );
}
