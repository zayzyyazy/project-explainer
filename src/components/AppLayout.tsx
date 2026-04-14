import { Link, Outlet } from "react-router-dom";

export default function AppLayout() {
  return (
    <div className="app-shell">
      <header className="top-bar">
        <Link
          to="/dashboard"
          style={{ textDecoration: "none", color: "inherit" }}
        >
          <h1>Project Explainer OS</h1>
        </Link>

        <nav className="top-nav">
          <Link to="/dashboard">Dashboard</Link>
          <Link to="/setup">Profile</Link>
          <Link to="/case-study">Case Study</Link>
          <Link to="/opportunities">Opportunities</Link>
          <Link to="/idea-projects">Idea Projects</Link>
        </nav>
      </header>

      <main>
        <Outlet />
      </main>
    </div>
  );
}
