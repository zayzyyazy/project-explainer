import { Link } from "react-router-dom";

export default function Landing() {
  return (
    <div className="landing">
      <header className="landing-hero">
        <p className="landing-badge">Desktop · BYOK · Local folders</p>
        <h1 className="landing-title">
          Turn finished repos into{" "}
          <span className="landing-accent">client-winning case studies</span>
        </h1>
        <p className="landing-sub">
          Scan a project folder, get Problem → Solution → Outcome plus
          proposal- and LinkedIn-ready copy. Built for freelancers who ship
          well but sell weak. You bring your own OpenAI or Anthropic key.
        </p>
        <div className="landing-cta-row">
          <Link to="/dashboard" className="btn btn-primary landing-cta-main">
            Open app
          </Link>
          <span className="landing-price">One-time-style workflow · your API usage</span>
        </div>
      </header>

      <section className="landing-section">
        <h2>Before / After</h2>
        <div className="landing-before-after">
          <div className="landing-ba card">
            <h3 className="landing-ba-label muted">Before</h3>
            <p>
              “It&apos;s a React app with Tauri and SQLite, has import and
              reanalyze…”
            </p>
            <p className="muted" style={{ marginTop: "0.75rem" }}>
              Technical laundry list. No story. Clients glaze over.
            </p>
          </div>
          <div className="landing-ba card landing-ba-after">
            <h3 className="landing-ba-label">After</h3>
            <p>
              “We gave developers a local tool that turns codebases into
              structured intelligence—so they stop rewriting the same
              explanations and can pitch past work with clarity.”
            </p>
            <p className="muted" style={{ marginTop: "0.75rem" }}>
              Business language, grounded in what shipped.
            </p>
          </div>
        </div>
      </section>

      <section className="landing-section">
        <h2>What you get</h2>
        <ul className="landing-bullets">
          <li>
            <strong>Grounded output</strong> — tied to files and patterns, not
            fantasy features
          </li>
          <li>
            <strong>Case study mode</strong> — narrative you can paste into
            proposals and portfolios
          </li>
          <li>
            <strong>Opportunities mode</strong> — narrow ways to package what
            you already built
          </li>
          <li>
            <strong>Private</strong> — runs on your machine; you control API
            spend
          </li>
        </ul>
      </section>

      <footer className="landing-footer">
        <Link to="/dashboard" className="btn btn-primary">
          Start with your projects
        </Link>
      </footer>
    </div>
  );
}
