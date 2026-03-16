import type { ReactNode } from "react";

export function StatCard({
  label,
  value,
  helper,
  icon,
}: {
  label: string;
  value: string;
  helper?: string;
  icon?: ReactNode;
}) {
  return (
    <div className="rounded-[24px] border border-black/10 bg-white/80 p-5 shadow-panel">
      <div className="flex items-center justify-between">
        <p className="text-xs uppercase tracking-[0.24em] text-steel">{label}</p>
        <span className="text-olive">{icon}</span>
      </div>
      <div className="mt-4 text-3xl font-semibold text-graphite">{value}</div>
      {helper ? <p className="mt-2 text-sm text-graphite/65">{helper}</p> : null}
    </div>
  );
}
