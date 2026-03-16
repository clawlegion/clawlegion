import { render, screen } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { describe, expect, it, vi } from "vitest";

import { I18nProvider } from "../i18n";
import { DashboardPage } from "./dashboard";

vi.mock("../hooks/use-api", () => ({
  useDashboardData: () => ({
    system: { data: { status: "healthy", uptime_secs: 42, agents_active: 4, agents_total: 5, plugins_loaded: 3, memory_usage_mb: 512 } },
    health: { data: { checks: { database: "healthy", llm_provider: "healthy", plugin_system: "degraded" } } },
    budget: { data: { budget_remaining_cents: 123400, usage_percentage: 48.6 } },
    agents: { data: { agents: [{ id: "a-1", name: "Alpha", role: "lead", title: "Commander", status: "active" }] } },
  }),
}));

describe("DashboardPage", () => {
  it("renders system and budget summaries", () => {
    const client = new QueryClient();
    render(
      <I18nProvider>
        <QueryClientProvider client={client}>
          <DashboardPage />
        </QueryClientProvider>
      </I18nProvider>,
    );

    expect(screen.getAllByText("healthy").length).toBeGreaterThan(0);
    expect(screen.getByText(/Active Agents/)).toBeInTheDocument();
    expect(screen.getByText(/Alpha/)).toBeInTheDocument();
  });
});
