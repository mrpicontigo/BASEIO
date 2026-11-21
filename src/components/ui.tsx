import type { ReactNode } from "react";
import type { AppError, Tier } from "../lib/types";
import { tierClass, tierLabel } from "../lib/format";

export function Card({ title, children, right }: { title?: string; children: ReactNode; right?: ReactNode }) {
  return (
    <section className="rounded-lg border border-line bg-panel p-4">
      {(title || right) && (
        <div className="mb-3 flex items-center justify-between">
          {title && <h2 className="text-sm font-semibold uppercase tracking-wide text-muted">{title}</h2>}
          {right}
        </div>
      )}
      {children}
    </section>
  );
}

export function TierBadge({ tier }: { tier: Tier }) {
  return (
    <span className={`inline-block rounded border px-2 py-0.5 text-xs font-medium ${tierClass[tier]}`}>
      {tierLabel[tier]}
    </span>
  );
}

export function ErrorBox({ error, onRetry }: { error: AppError; onRetry?: () => void }) {
  return (
    <div className="rounded-lg border border-bad/40 bg-bad/10 p-4 text-sm">
      <p className="font-medium text-bad">Something went wrong</p>
      <p className="mt-1 text-fg/80">{error.message}</p>
      {onRetry && (
        <button onClick={onRetry} className="mt-3 rounded bg-panel-2 px-3 py-1.5 text-xs hover:bg-line">
          Try again
        </button>
      )}
    </div>
  );
}

export function Loading({ label = "Loading…" }: { label?: string }) {
  return <div className="p-6 text-sm text-muted">{label}</div>;
}

export function Meter({ used, total }: { used: number; total: number }) {
  const pct = total > 0 ? Math.min(100, (used / total) * 100) : 0;
  return (
    <div className="h-2 w-full overflow-hidden rounded bg-panel-2">
      <div className="h-full rounded bg-accent" style={{ width: `${pct}%` }} />
    </div>
  );
}

export function Button({
  children, onClick, variant = "primary", disabled, title,
}: {
  children: ReactNode; onClick?: () => void; variant?: "primary" | "ghost" | "danger";
  disabled?: boolean; title?: string;
}) {
  const styles = {
    primary: "bg-accent text-white hover:bg-accent/85",
    ghost: "bg-panel-2 text-fg hover:bg-line",
    danger: "bg-bad/80 text-white hover:bg-bad",
  }[variant];
  return (
    <button
      title={title}
      disabled={disabled}
      onClick={onClick}
      className={`rounded px-3 py-1.5 text-sm font-medium transition disabled:cursor-not-allowed disabled:opacity-50 ${styles}`}
    >
      {children}
    </button>
  );
}
