import { useMemo, useState } from "react";
import * as Dialog from "@radix-ui/react-dialog";
import { Search } from "lucide-react";

import { useAgents } from "../hooks/use-api";
import { currencyFromCents, formatDateTime } from "../lib/utils";
import { StatusPill } from "../components/status-pill";
import { SectionCard } from "../components/section-card";
import { useI18n } from "../i18n";
import { AgentDetailPanel } from "./components/agent-detail-panel";

export function AgentsPage() {
  const agents = useAgents();
  const [query, setQuery] = useState("");
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const { t, intlLocale } = useI18n();

  const filtered = useMemo(() => {
    const items = agents.data?.agents ?? [];
    return items.filter((agent) => {
      const text = `${agent.name} ${agent.role} ${agent.title}`.toLowerCase();
      return text.includes(query.toLowerCase());
    });
  }, [agents.data?.agents, query]);

  return (
    <SectionCard title={t("agents.title")} subtitle={t("agents.subtitle")}>
      <div className="mb-4 flex items-center gap-3 rounded-2xl border border-black/10 bg-stone-50 px-4 py-3">
        <Search className="h-4 w-4 text-steel" />
        <input
          className="w-full bg-transparent text-sm outline-none"
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          placeholder={t("agents.search")}
        />
      </div>

      <div className="overflow-hidden rounded-[24px] border border-black/10">
        <table className="min-w-full divide-y divide-black/10 text-sm">
          <thead className="bg-stone-100 text-left text-xs uppercase tracking-[0.24em] text-steel">
            <tr>
              <th className="px-4 py-3">{t("agents.table.agent")}</th>
              <th className="px-4 py-3">{t("agents.table.status")}</th>
              <th className="px-4 py-3">{t("agents.table.heartbeat")}</th>
              <th className="px-4 py-3">{t("agents.table.budget")}</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-black/5 bg-white/80">
            {filtered.map((agent) => (
              <tr
                key={agent.id}
                className="cursor-pointer hover:bg-stone-50"
                onClick={() => setSelectedId(agent.id)}
              >
                <td className="px-4 py-4">
                  <p className="font-medium">{agent.name}</p>
                  <p className="text-graphite/60">{agent.role} - {agent.title}</p>
                </td>
                <td className="px-4 py-4"><StatusPill status={agent.status} /></td>
                <td className="px-4 py-4 text-graphite/70">{formatDateTime(agent.last_heartbeat, intlLocale)}</td>
                <td className="px-4 py-4 text-graphite/70">{currencyFromCents(agent.budget_remaining, intlLocale)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      <Dialog.Root open={Boolean(selectedId)} onOpenChange={(open) => !open && setSelectedId(null)}>
        <Dialog.Portal>
          <Dialog.Overlay className="fixed inset-0 bg-black/30 backdrop-blur-sm" />
          <Dialog.Content className="fixed right-4 top-4 h-[calc(100vh-2rem)] w-full max-w-2xl overflow-auto rounded-[28px] bg-sand p-6 shadow-panel">
            {selectedId ? <AgentDetailPanel agentId={selectedId} /> : null}
          </Dialog.Content>
        </Dialog.Portal>
      </Dialog.Root>
    </SectionCard>
  );
}
