import { Activity, Cpu, PlugZap } from "lucide-react";

import { useDashboardData } from "../hooks/use-api";
import { SectionCard } from "../components/section-card";
import { StatCard } from "../components/stat-card";
import { StatusPill } from "../components/status-pill";
import { useI18n } from "../i18n";

export function DashboardPage() {
  const { agents, health, system } = useDashboardData();
  const { t } = useI18n();
  const isLoading = agents.isLoading || health.isLoading || system.isLoading;
  const loadError = agents.error ?? health.error ?? system.error;

  return (
    <div className="space-y-5">
      {isLoading ? (
        <div className="rounded-2xl border border-dashed border-black/10 bg-stone-50 px-4 py-5 text-sm text-graphite/60">
          {t("messages.loading")}
        </div>
      ) : null}
      {loadError ? (
        <div className="rounded-2xl border border-amber-300 bg-amber-50 px-4 py-5 text-sm text-amber-900">
          {t("messages.loadError")}
        </div>
      ) : null}
      <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-3">
        <StatCard
          label={t("dashboard.systemStatus")}
          value={system.data?.status ?? t("common.na")}
          helper={t("dashboard.uptime", { seconds: system.data?.uptime_secs ?? 0 })}
          icon={<Activity className="h-5 w-5" />}
        />
        <StatCard
          label={t("dashboard.agentActive")}
          value={`${system.data?.agents_active ?? t("common.na")}/${system.data?.agents_total ?? t("common.na")}`}
          helper={t("dashboard.polling")}
          icon={<Cpu className="h-5 w-5" />}
        />
        <StatCard
          label={t("dashboard.pluginsLoaded")}
          value={`${system.data?.plugins_loaded ?? t("common.na")}`}
          helper={t("dashboard.memory", { value: system.data?.memory_usage_mb ?? t("common.na") })}
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
