import type { PropsWithChildren, ReactNode } from "react";

export function SectionCard({
  title,
  subtitle,
  actions,
  children,
}: PropsWithChildren<{
  title: string;
  subtitle?: string;
  actions?: ReactNode;
}>) {
  return (
    <section className="rounded-[28px] border border-black/10 bg-white/75 p-5 shadow-panel backdrop-blur">
      <header className="mb-4 flex items-start justify-between gap-4">
        <div>
          <p className="text-xs uppercase tracking-[0.24em] text-steel">{title}</p>
          {subtitle ? <h2 className="mt-2 text-sm text-graphite/70">{subtitle}</h2> : null}
        </div>
        {actions}
      </header>
      {children}
    </section>
  );
}
