import { Routes, Route, Link } from "react-router-dom";

import Dashboard from "./pages/Dashboard";
import IdeaProjects from "./pages/IdeaProjects";
import Opportunities from "./pages/Opportunities";
import ProjectDetail from "./pages/ProjectDetail";

export default function App() {
  return (
    <div className="app-shell">
      <header className="top-bar">
        <Link to="/" style={{ textDecoration: "none", color: "inherit" }}>
          <h1>Project Explainer OS</h1>
        </Link>

        <nav className="top-nav">
          <Link to="/">Dashboard</Link>
          <Link to="/opportunities">Opportunities</Link>
          <Link to="/idea-projects">Idea Projects</Link>
        </nav>
      </header>

      <main>
        <Routes>
          <Route path="/" element={<Dashboard />} />
          <Route path="/opportunities" element={<Opportunities />} />
          <Route path="/idea-projects" element={<IdeaProjects />} />
          <Route path="/project/:id" element={<ProjectDetail />} />
        </Routes>
      </main>
    </div>
  );
}