import { Activity, Building2, MessagesSquare, Radar, Server, Users } from "lucide-react";
import { NavLink, Outlet } from "react-router-dom";

import { cn } from "../lib/utils";
import { useSystemStatus } from "../hooks/use-api";
import { useI18n } from "../i18n";
import { StatusPill } from "./status-pill";

const NAV_ITEMS = [
  { to: "/dashboard", labelKey: "app.nav.dashboard", icon: Radar },
  { to: "/agents", labelKey: "app.nav.agents", icon: Users },
  { to: "/org", labelKey: "app.nav.org", icon: Building2 },
  { to: "/messages", labelKey: "app.nav.messages", icon: MessagesSquare },
  { to: "/system", labelKey: "app.nav.system", icon: Server },
];

export function AppShell() {
  const system = useSystemStatus();
  const { locale, localeOptions, setLocale, t } = useI18n();

  return (
    <div className="min-h-screen bg-grain text-graphite">
      <div className="mx-auto flex min-h-screen max-w-[1600px] gap-6 px-4 py-4 lg:px-6">
        <aside className="hidden w-72 shrink-0 rounded-[32px] border border-black/10 bg-graphite p-6 text-sand shadow-panel lg:block">
          <div>
            <p className="text-xs uppercase tracking-[0.3em] text-sand/60">{t("app.brand")}</p>
            <h1 className="mt-3 text-3xl font-semibold">{t("app.console")}</h1>
            <p className="mt-3 text-sm leading-6 text-sand/70">
              {t("app.description")}
            </p>
          </div>
          <nav className="mt-10 space-y-2">
            {NAV_ITEMS.map((item) => (
              <NavLink
                key={item.to}
                to={item.to}
                className={({ isActive }) =>
                  cn(
                    "flex items-center gap-3 rounded-2xl px-4 py-3 text-sm transition",
                    isActive ? "bg-sand text-graphite" : "text-sand/70 hover:bg-white/10 hover:text-sand",
                  )
                }
              >
                <item.icon className="h-4 w-4" />
                {t(item.labelKey)}
              </NavLink>
            ))}
          </nav>
        </aside>
        <div className="flex min-w-0 flex-1 flex-col gap-5">
          <header className="rounded-[28px] border border-black/10 bg-white/70 px-5 py-4 shadow-panel backdrop-blur">
            <div className="flex flex-col gap-4 xl:flex-row xl:items-center xl:justify-between">
              <div>
                <p className="text-xs uppercase tracking-[0.25em] text-steel">{t("app.liveSummary")}</p>
                <div className="mt-3 flex flex-wrap items-center gap-4 text-sm text-graphite/80">
                  <span className="inline-flex items-center gap-2">
                    <Activity className="h-4 w-4 text-signal" />
                    {t("app.system")}
                    <StatusPill status={system.data?.status} />
                  </span>
                  <span>
                    {t("app.activeAgents", {
                      active: system.data?.agents_active ?? "--",
                      total: system.data?.agents_total ?? "--",
                    })}
                  </span>
                </div>
              </div>
              <div className="flex flex-wrap items-center justify-end gap-3 text-xs uppercase tracking-[0.2em] text-graphite/55">
                <label className="inline-flex items-center gap-2 rounded-full border border-black/10 px-3 py-2 normal-case tracking-normal text-graphite/70">
                  <span>{t("app.language")}</span>
                  <select
                    className="rounded-md border border-black/10 bg-white/85 px-2 py-1 text-xs text-graphite outline-none"
                    value={locale}
                    onChange={(event) => setLocale(event.target.value as typeof locale)}
                  >
                    {localeOptions.map((option) => (
                      <option key={option.code} value={option.code}>
                        {option.label}
                      </option>
                    ))}
                  </select>
                </label>
                <span className="rounded-full border border-black/10 px-3 py-2">{t("app.badge.hashRouter")}</span>
                <span className="rounded-full border border-black/10 px-3 py-2">{t("app.badge.polling")}</span>
                <span className="rounded-full border border-black/10 px-3 py-2">{t("app.badge.static")}</span>
              </div>
            </div>
          </header>
          <main className="pb-6">
            <Outlet />
          </main>
        </div>
      </div>
    </div>
  );
}
