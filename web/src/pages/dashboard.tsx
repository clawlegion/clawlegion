import { Activity, Cpu, DollarSign, PlugZap } from "lucide-react";

import { useDashboardData } from "../hooks/use-api";
import { currencyFromCents } from "../lib/utils";
import { SectionCard } from "../components/section-card";
import { StatCard } from "../components/stat-card";
import { StatusPill } from "../components/status-pill";
import { useI18n } from "../i18n";

export function DashboardPage() {
  const { agents, budget, health, system } = useDashboardData();
  const { t, intlLocale } = useI18n();

  return (
    <div className="space-y-5">
      <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
        <StatCard
          label={t("dashboard.systemStatus")}
          value={system.data?.status ?? "--"}
          helper={t("dashboard.uptime", { seconds: system.data?.uptime_secs ?? 0 })}
          icon={<Activity className="h-5 w-5" />}
        />
        <StatCard
          label={t("dashboard.agentActive")}
          value={`${system.data?.agents_active ?? "--"}/${system.data?.agents_total ?? "--"}`}
          helper={t("dashboard.polling")}
          icon={<Cpu className="h-5 w-5" />}
        />
        <StatCard
          label={t("dashboard.budgetRemaining")}
          value={budget.data ? currencyFromCents(budget.data.budget_remaining_cents, intlLocale) : "--"}
          helper={budget.data ? t("dashboard.usage", { percent: budget.data.usage_percentage.toFixed(1) }) : undefined}
          icon={<DollarSign className="h-5 w-5" />}
        />
        <StatCard
          label={t("dashboard.pluginsLoaded")}
          value={`${system.data?.plugins_loaded ?? "--"}`}
          helper={t("dashboard.memory", { value: system.data?.memory_usage_mb ?? "--" })}
          icon={<PlugZap className="h-5 w-5" />}
        />
      </div>

      <div className="grid gap-5 xl:grid-cols-[1.2fr_0.8fr]">
        <SectionCard title={t("dashboard.health.title")} subtitle={t("dashboard.health.subtitle")}>
          <div className="grid gap-3 md:grid-cols-3">
            {(["database", "llm_provider", "plugin_system"] as const).map((key) => (
              <div key={key} className="rounded-2xl border border-black/10 bg-stone-50 p-4">
                <p className="text-xs uppercase tracking-[0.24em] text-steel">{key}</p>
                <div className="mt-3">
                  <StatusPill status={health.data?.checks[key]} />
                </div>
              </div>
            ))}
          </div>
        </SectionCard>

        <SectionCard title={t("dashboard.snapshot.title")} subtitle={t("dashboard.snapshot.subtitle")}>
          <div className="space-y-3">
            {agents.data?.agents.slice(0, 6).map((agent) => (
              <div
                key={agent.id}
                className="flex items-center justify-between rounded-2xl border border-black/10 bg-stone-50 px-4 py-3"
              >
                <div>
                  <p className="font-medium">{agent.name}</p>
                  <p className="text-sm text-graphite/65">{agent.role} - {agent.title}</p>
                </div>
                <StatusPill status={agent.status} />
              </div>
            ))}
          </div>
        </SectionCard>
      </div>
    </div>
  );
}
