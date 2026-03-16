import { useAgent, useAgentSkills, useAgentStatus } from "../../hooks/use-api";
import { currencyFromCents, formatDateTime } from "../../lib/utils";
import { SectionCard } from "../../components/section-card";
import { StatusPill } from "../../components/status-pill";
import { useI18n } from "../../i18n";

export function AgentDetailPanel({ agentId }: { agentId: string }) {
  const detail = useAgent(agentId);
  const status = useAgentStatus(agentId);
  const skills = useAgentSkills(agentId);
  const { t, intlLocale } = useI18n();

  if (detail.isLoading) {
    return <div className="p-8 text-sm text-graphite/70">{t("agent.loading")}</div>;
  }

  if (!detail.data) {
    return <div className="p-8 text-sm text-rose-700">{t("agent.notFound")}</div>;
  }

  return (
    <div className="space-y-5">
      <header>
        <p className="text-xs uppercase tracking-[0.24em] text-steel">{t("agent.header")}</p>
        <h2 className="mt-2 text-3xl font-semibold">{detail.data.name}</h2>
        <p className="mt-2 text-graphite/70">{detail.data.role} - {detail.data.title}</p>
      </header>
      <div className="grid gap-4 md:grid-cols-2">
        <SectionCard title={t("agent.runtime.title")} subtitle={t("agent.runtime.subtitle")}>
          <div className="space-y-3 text-sm">
            <div className="flex items-center justify-between"><span>{t("agent.status")}</span><StatusPill status={status.data?.status ?? detail.data.status} /></div>
            <div className="flex items-center justify-between"><span>{t("agent.currentTask")}</span><span>{status.data?.current_task ?? t("agent.idle")}</span></div>
            <div className="flex items-center justify-between"><span>{t("agent.lastHeartbeat")}</span><span>{formatDateTime(status.data?.last_heartbeat ?? detail.data.last_heartbeat, intlLocale)}</span></div>
          </div>
        </SectionCard>
        <SectionCard title={t("agent.cost.title")} subtitle={t("agent.cost.subtitle")}>
          <div className="space-y-3 text-sm">
            <div className="flex items-center justify-between"><span>{t("agent.budgetRemaining")}</span><span>{currencyFromCents(detail.data.budget_remaining, intlLocale)}</span></div>
            <div className="flex items-center justify-between"><span>{t("agent.tokenTotal")}</span><span>{detail.data.token_usage_total.toLocaleString(intlLocale)}</span></div>
            <div className="flex items-center justify-between"><span>{t("agent.totalCost")}</span><span>{currencyFromCents(detail.data.cost_total_cents, intlLocale)}</span></div>
          </div>
        </SectionCard>
      </div>
      <SectionCard title={t("agent.skills.title")} subtitle={t("agent.skills.subtitle")}>
        <div className="grid gap-4 md:grid-cols-2">
          <div>
            <p className="mb-2 text-xs uppercase tracking-[0.24em] text-steel">{t("agent.capabilities")}</p>
            <div className="flex flex-wrap gap-2">
              {detail.data.capabilities.map((capability) => (
                <span key={capability} className="rounded-full border border-black/10 px-3 py-1 text-sm">
                  {capability}
                </span>
              ))}
            </div>
          </div>
          <div>
            <p className="mb-2 text-xs uppercase tracking-[0.24em] text-steel">{t("agent.skills")}</p>
            <div className="space-y-2">
              {skills.data?.skills.map((skill) => (
                <div key={`${skill.name}-${skill.version}`} className="rounded-2xl border border-black/10 bg-stone-50 px-4 py-3">
                  <p className="font-medium">{skill.name} <span className="text-graphite/50">v{skill.version}</span></p>
                  <p className="text-sm text-graphite/65">{skill.description}</p>
                </div>
              ))}
            </div>
          </div>
        </div>
      </SectionCard>
    </div>
  );
}
