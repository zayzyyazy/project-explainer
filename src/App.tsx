import { Routes, Route, Link } from "react-router-dom";
import Dashboard from "./pages/Dashboard";
import Opportunities from "./pages/Opportunities";
import ProjectDetail from "./pages/ProjectDetail";

export default function App() {
  return (
    <div className="app-shell">
      <div className="top-bar">
        <Link to="/">
          <h1>Project Explainer OS</h1>
        </Link>
        <nav style={{ display: "flex", gap: "1rem", alignItems: "center" }}>
          <Link to="/opportunities">Opportunities</Link>
        </nav>
      </div>
      <Routes>
        <Route path="/" element={<Dashboard />} />
        <Route path="/opportunities" element={<Opportunities />} />
        <Route path="/project/:id" element={<ProjectDetail />} />
      </Routes>
    </div>
  );
}
