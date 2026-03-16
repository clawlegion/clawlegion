import { cn } from "../lib/utils";

const COLOR_MAP: Record<string, string> = {
  healthy: "bg-emerald-100 text-emerald-800 border-emerald-300",
  active: "bg-emerald-100 text-emerald-800 border-emerald-300",
  idle: "bg-amber-100 text-amber-800 border-amber-300",
  degraded: "bg-orange-100 text-orange-800 border-orange-300",
  down: "bg-rose-100 text-rose-800 border-rose-300",
};

export function StatusPill({ status }: { status?: string | null }) {
  const normalized = status?.toLowerCase() ?? "unknown";
  return (
    <span
      className={cn(
        "inline-flex rounded-full border px-2 py-1 text-xs font-semibold uppercase tracking-[0.18em]",
        COLOR_MAP[normalized] ?? "bg-zinc-100 text-zinc-700 border-zinc-300",
      )}
    >
      {normalized}
    </span>
  );
}
