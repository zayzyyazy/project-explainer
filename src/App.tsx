import { Routes, Route } from "react-router-dom";

import AppLayout from "./components/AppLayout";
import CaseStudy from "./pages/CaseStudy";
import Dashboard from "./pages/Dashboard";
import IdeaProjects from "./pages/IdeaProjects";
import Landing from "./pages/Landing";
import Opportunities from "./pages/Opportunities";
import ProjectDetail from "./pages/ProjectDetail";

export default function App() {
  return (
    <Routes>
      <Route path="/" element={<Landing />} />
      <Route element={<AppLayout />}>
        <Route path="dashboard" element={<Dashboard />} />
        <Route path="case-study" element={<CaseStudy />} />
        <Route path="opportunities" element={<Opportunities />} />
        <Route path="idea-projects" element={<IdeaProjects />} />
        <Route path="project/:id" element={<ProjectDetail />} />
      </Route>
    </Routes>
  );
}
