import { expect, test } from "@playwright/test";

test("dashboard shell loads", async ({ page }) => {
  await page.route("**/api/**", async (route) => {
    const url = route.request().url();
    if (url.endsWith("/system/status")) {
      await route.fulfill({ json: { status: "healthy", uptime_secs: 42, version: "dev", agents_total: 4, agents_active: 3, plugins_loaded: 2, memory_usage_mb: 128 } });
      return;
    }
    if (url.endsWith("/system/health")) {
      await route.fulfill({ json: { status: "healthy", checks: { database: "healthy", llm_provider: "healthy", plugin_system: "healthy" } } });
      return;
    }
    if (url.endsWith("/org/budget")) {
      await route.fulfill({ json: { company_id: "c-1", budget_monthly_cents: 100000, budget_spent_cents: 30000, budget_remaining_cents: 70000, usage_percentage: 30, projected_overrun: false, top_spenders: [] } });
      return;
    }
    if (url.endsWith("/agents")) {
      await route.fulfill({ json: { agents: [{ id: "a1", name: "Alpha", role: "lead", title: "Commander", status: "active", budget_remaining: 1000, token_usage_total: 1, cost_total_cents: 1 }] } });
      return;
    }
    await route.fulfill({ json: {} });
  });

  await page.goto("/#/dashboard");
  await expect(page.getByText("Command Console")).toBeVisible();
  await expect(page.getByText("Agent 活跃数")).toBeVisible();
});
