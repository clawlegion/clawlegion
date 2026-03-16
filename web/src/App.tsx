import { HashRouter, Navigate, Route, Routes } from "react-router-dom";

import { AppShell } from "./components/app-shell";
import { AgentsPage } from "./pages/agents";
import { DashboardPage } from "./pages/dashboard";
import { MessagesPage } from "./pages/messages";
import { OrgPage } from "./pages/org";
import { SystemPage } from "./pages/system";

export function App() {
  return (
    <HashRouter>
      <Routes>
        <Route element={<AppShell />}>
          <Route path="/" element={<Navigate to="/dashboard" replace />} />
          <Route path="/dashboard" element={<DashboardPage />} />
          <Route path="/agents" element={<AgentsPage />} />
          <Route path="/org" element={<OrgPage />} />
          <Route path="/messages" element={<MessagesPage />} />
          <Route path="/system" element={<SystemPage />} />
        </Route>
      </Routes>
    </HashRouter>
  );
}
