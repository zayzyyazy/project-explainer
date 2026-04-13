import { Routes, Route, Link } from "react-router-dom";
import Dashboard from "./pages/Dashboard";
import ProjectDetail from "./pages/ProjectDetail";

export default function App() {
  return (
    <div className="app-shell">
      <div className="top-bar">
        <Link to="/">
          <h1>Project Explainer OS</h1>
        </Link>
      </div>
      <Routes>
        <Route path="/" element={<Dashboard />} />
        <Route path="/project/:id" element={<ProjectDetail />} />
      </Routes>
    </div>
  );
}
