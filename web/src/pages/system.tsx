import { useState } from "react";

import {
  useDisablePlugin,
  useEnablePlugin,
  useInstallPlugin,
  usePlugins,
  usePluginDoctor,
  usePluginLogs,
  useReloadPlugin,
  useSignPlugin,
  useSystemHealth,
  useSystemStatus,
  useTrustPluginKey,
  useUninstallPlugin,
} from "../hooks/use-api";
import { formatDateTime } from "../lib/utils";
import { SectionCard } from "../components/section-card";
import { StatusPill } from "../components/status-pill";
import { useI18n } from "../i18n";

export function SystemPage() {
  const status = useSystemStatus();
  const health = useSystemHealth();
  const plugins = usePlugins();
  const pluginDoctor = usePluginDoctor();
  const enablePlugin = useEnablePlugin();
  const disablePlugin = useDisablePlugin();
  const reloadPlugin = useReloadPlugin();
  const installPlugin = useInstallPlugin();
  const uninstallPlugin = useUninstallPlugin();
  const trustPluginKey = useTrustPluginKey();
  const signPlugin = useSignPlugin();
  const [selectedPluginId, setSelectedPluginId] = useState("");
  const pluginLogs = usePluginLogs(selectedPluginId || plugins.data?.plugins[0]?.id);
  const [installPath, setInstallPath] = useState("");
  const [trustAlias, setTrustAlias] = useState("");
  const [trustPath, setTrustPath] = useState("");
  const [signPath, setSignPath] = useState("");
  const { t, intlLocale } = useI18n();

  return (
    <div className="grid gap-5 xl:grid-cols-[0.78fr_1.22fr]">
      <div className="space-y-5">
        <SectionCard title={t("system.title")} subtitle={t("system.subtitle")}>
          <div className="space-y-4 text-sm">
            <div className="flex items-center justify-between">
              <span>{t("system.status")}</span>
              <StatusPill status={status.data?.status} />
            </div>
            <div className="flex items-center justify-between">
              <span>{t("system.version")}</span>
              <span>{status.data?.version ?? "--"}</span>
            </div>
            <div className="flex items-center justify-between">
              <span>{t("system.uptime")}</span>
              <span>
                {status.data?.uptime_secs ?? "--"} {t("common.seconds")}
              </span>
            </div>
            <div className="flex items-center justify-between">
              <span>{t("system.memory")}</span>
              <span>{status.data?.memory_usage_mb ?? "--"} MB</span>
            </div>
            <div className="flex items-center justify-between">
              <span>Plugins</span>
              <span>
                {status.data?.plugins_active ?? 0} / {status.data?.plugins_loaded ?? 0}
              </span>
            </div>
          </div>
        </SectionCard>

        <SectionCard title={t("system.health.title")} subtitle={t("system.health.subtitle")}>
          <div className="grid gap-4 md:grid-cols-3">
            {(["database", "llm_provider", "plugin_system"] as const).map((key) => (
              <div key={key} className="rounded-2xl border border-black/10 bg-stone-50 p-4">
                <p className="text-xs uppercase tracking-[0.24em] text-steel">{key}</p>
                <div className="mt-3">
                  <StatusPill status={health.data?.checks[key]} />
                </div>
                <p className="mt-3 text-xs text-graphite/55">
                  {t("common.lastPolled", {
                    time: formatDateTime(new Date().toISOString(), intlLocale),
                  })}
                </p>
              </div>
            ))}
          </div>
        </SectionCard>
      </div>

      <SectionCard title="Plugin Platform" subtitle="Manifest-backed discovery, state, and capability index">
        <div className="space-y-4">
          <div className="grid gap-3 md:grid-cols-3">
            <div className="rounded-2xl border border-black/10 bg-stone-50 p-4">
              <p className="text-xs uppercase tracking-[0.24em] text-steel">Healthy</p>
              <p className="mt-2 font-mono text-2xl">{status.data?.plugin_health.healthy ?? 0}</p>
            </div>
            <div className="rounded-2xl border border-black/10 bg-stone-50 p-4">
              <p className="text-xs uppercase tracking-[0.24em] text-steel">Degraded</p>
              <p className="mt-2 font-mono text-2xl">{status.data?.plugin_health.degraded ?? 0}</p>
            </div>
            <div className="rounded-2xl border border-black/10 bg-stone-50 p-4">
              <p className="text-xs uppercase tracking-[0.24em] text-steel">Failed</p>
              <p className="mt-2 font-mono text-2xl">{status.data?.plugin_health.failed ?? 0}</p>
            </div>
          </div>

          <div className="rounded-3xl border border-black/10">
            <div className="grid grid-cols-[1.1fr_0.8fr_0.8fr_0.5fr_0.8fr] gap-3 border-b border-black/10 px-4 py-3 text-xs uppercase tracking-[0.24em] text-steel">
              <span>Plugin</span>
              <span>Runtime</span>
              <span>State</span>
              <span>Caps</span>
              <span>Actions</span>
            </div>
            <div className="divide-y divide-black/10">
              {plugins.data?.plugins.map((plugin) => (
                <div key={plugin.id} className="grid grid-cols-[1.1fr_0.8fr_0.8fr_0.5fr_0.8fr] gap-3 px-4 py-3 text-sm">
                  <div>
                    <p className="font-medium">{plugin.id}</p>
                    <p className="text-xs text-graphite/55">
                      {plugin.enabled ? "enabled" : "disabled"}
                    </p>
                  </div>
                  <span className="capitalize">{plugin.manifest.runtime}</span>
                  <div>
                    <StatusPill status={plugin.state.toLowerCase()} />
                  </div>
                  <span>{plugin.manifest.capabilities.length}</span>
                  <div className="flex flex-wrap gap-2">
                    <button
                      className="rounded-full border border-black/10 px-2 py-1 text-xs"
                      onClick={() =>
                        plugin.enabled
                          ? disablePlugin.mutate(plugin.id)
                          : enablePlugin.mutate(plugin.id)
                      }
                    >
                      {plugin.enabled ? "Disable" : "Enable"}
                    </button>
                    <button
                      className="rounded-full border border-black/10 px-2 py-1 text-xs"
                      onClick={() => reloadPlugin.mutate(plugin.id)}
                    >
                      Reload
                    </button>
                    <button
                      className="rounded-full border border-black/10 px-2 py-1 text-xs"
                      onClick={() => uninstallPlugin.mutate(plugin.id)}
                    >
                      Uninstall
                    </button>
                  </div>
                </div>
              )) ?? (
                <div className="px-4 py-6 text-sm text-graphite/60">No plugins discovered.</div>
              )}
            </div>
          </div>

          <div className="rounded-2xl border border-black/10 bg-stone-50 p-4">
            <p className="text-xs uppercase tracking-[0.24em] text-steel">Capability Index</p>
            <div className="mt-3 flex flex-wrap gap-2">
              {Object.entries(plugins.data?.capability_index ?? {}).map(([kind, owners]) => (
                <span
                  key={kind}
                  className="rounded-full border border-black/10 bg-white px-3 py-1 text-xs text-graphite"
                >
                  {kind}: {owners.length}
                </span>
              ))}
            </div>
          </div>

          <div className="rounded-2xl border border-black/10 bg-stone-50 p-4">
            <p className="text-xs uppercase tracking-[0.24em] text-steel">Bridge Index</p>
            <div className="mt-3 flex flex-wrap gap-2">
              {Object.entries(plugins.data?.bridge_index ?? {}).map(([kind, owners]) => (
                <span
                  key={kind}
                  className="rounded-full border border-black/10 bg-white px-3 py-1 text-xs text-graphite"
                >
                  {kind}: {owners.length}
                </span>
              ))}
            </div>
          </div>

          <div className="rounded-2xl border border-black/10 bg-stone-50 p-4">
            <p className="text-xs uppercase tracking-[0.24em] text-steel">Sentinel Triggers</p>
            <div className="mt-3 space-y-2">
              {(plugins.data?.sentinel_triggers ?? []).map((trigger) => (
                <div
                  key={`${trigger.id}:${trigger.agent_id}`}
                  className="rounded-2xl border border-black/10 bg-white px-3 py-2 text-sm text-graphite"
                >
                  <p className="font-medium">{trigger.id}</p>
                  <p className="text-xs text-steel">agent: {trigger.agent_id}</p>
                  <p className="text-xs text-steel">enabled: {String(trigger.enabled)}</p>
                  <p className="text-xs text-steel break-all">condition: {trigger.condition}</p>
                </div>
              ))}
              {!(plugins.data?.sentinel_triggers?.length ?? 0) && (
                <p className="text-sm text-steel">No sentinel triggers registered.</p>
              )}
            </div>
          </div>

          <div className="rounded-2xl border border-black/10 bg-stone-50 p-4">
            <p className="text-xs uppercase tracking-[0.24em] text-steel">Plugin Doctor</p>
            <div className="mt-3 space-y-2">
              {(pluginDoctor.data?.reports ?? []).map((report, index) => (
                <pre
                  key={index}
                  className="overflow-auto rounded-2xl border border-black/10 bg-white p-3 text-xs text-graphite"
                >
                  {JSON.stringify(report, null, 2)}
                </pre>
              ))}
              {!(pluginDoctor.data?.reports?.length ?? 0) && (
                <p className="text-sm text-steel">No doctor report.</p>
              )}
            </div>
          </div>

          <div className="rounded-2xl border border-black/10 bg-stone-50 p-4">
            <p className="text-xs uppercase tracking-[0.24em] text-steel">Plugin Logs</p>
            <select
              className="mt-3 w-full rounded-xl border border-black/10 bg-white px-3 py-2 text-sm"
              value={selectedPluginId}
              onChange={(event) => setSelectedPluginId(event.target.value)}
            >
              <option value="">Auto (first plugin)</option>
              {(plugins.data?.plugins ?? []).map((plugin) => (
                <option key={plugin.id} value={plugin.id}>
                  {plugin.id}
                </option>
              ))}
            </select>
            <div className="mt-3 space-y-2">
              {(pluginLogs.data?.logs ?? []).map((line, idx) => (
                <div
                  key={`${line.runtime}:${idx}`}
                  className="rounded-xl border border-black/10 bg-white px-3 py-2 text-xs"
                >
                  <span className="font-mono text-steel">[{line.runtime}]</span>{" "}
                  <span>{line.message}</span>
                </div>
              ))}
              {!(pluginLogs.data?.logs?.length ?? 0) && (
                <p className="text-sm text-steel">No runtime logs.</p>
              )}
            </div>
          </div>

          <div className="grid gap-4 md:grid-cols-3">
            <form
              className="rounded-2xl border border-black/10 bg-stone-50 p-4"
              onSubmit={(event) => {
                event.preventDefault();
                if (installPath.trim()) {
                  installPlugin.mutate(installPath.trim());
                  setInstallPath("");
                }
              }}
            >
              <p className="text-xs uppercase tracking-[0.24em] text-steel">Install Plugin</p>
              <input
                className="mt-3 w-full rounded-xl border border-black/10 bg-white px-3 py-2 text-sm"
                placeholder="/path/to/plugin"
                value={installPath}
                onChange={(event) => setInstallPath(event.target.value)}
              />
              <button className="mt-3 rounded-full border border-black/10 px-3 py-1 text-xs">
                Install
              </button>
            </form>

            <form
              className="rounded-2xl border border-black/10 bg-stone-50 p-4"
              onSubmit={(event) => {
                event.preventDefault();
                if (trustAlias.trim() && trustPath.trim()) {
                  trustPluginKey.mutate({
                    alias: trustAlias.trim(),
                    publicKeyPath: trustPath.trim(),
                  });
                  setTrustAlias("");
                  setTrustPath("");
                }
              }}
            >
              <p className="text-xs uppercase tracking-[0.24em] text-steel">Trust Key</p>
              <input
                className="mt-3 w-full rounded-xl border border-black/10 bg-white px-3 py-2 text-sm"
                placeholder="alias"
                value={trustAlias}
                onChange={(event) => setTrustAlias(event.target.value)}
              />
              <input
                className="mt-2 w-full rounded-xl border border-black/10 bg-white px-3 py-2 text-sm"
                placeholder="/path/to/public.key"
                value={trustPath}
                onChange={(event) => setTrustPath(event.target.value)}
              />
              <button className="mt-3 rounded-full border border-black/10 px-3 py-1 text-xs">
                Trust
              </button>
            </form>

            <form
              className="rounded-2xl border border-black/10 bg-stone-50 p-4"
              onSubmit={(event) => {
                event.preventDefault();
                const target = selectedPluginId || plugins.data?.plugins[0]?.id;
                if (target && signPath.trim()) {
                  signPlugin.mutate({ id: target, privateKeyPath: signPath.trim() });
                  setSignPath("");
                }
              }}
            >
              <p className="text-xs uppercase tracking-[0.24em] text-steel">Sign First Plugin</p>
              <input
                className="mt-3 w-full rounded-xl border border-black/10 bg-white px-3 py-2 text-sm"
                placeholder="/path/to/private.key"
                value={signPath}
                onChange={(event) => setSignPath(event.target.value)}
              />
              <button className="mt-3 rounded-full border border-black/10 px-3 py-1 text-xs">
                Sign
              </button>
            </form>
          </div>
        </div>
      </SectionCard>
    </div>
  );
}
