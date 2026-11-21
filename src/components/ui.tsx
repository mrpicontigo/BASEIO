import type { ReactNode } from "react";
import type { AppError, Tier } from "../lib/types";
import { tierClass, tierLabel } from "../lib/format";

export function Card({ title, children, right }: { title?: string; children: ReactNode; right?: ReactNode }) {
  return (
    <section className="rounded-2xl border border-line bg-panel p-5 backdrop-blur-md">
      {(title || right) && (
        <div className="mb-4 flex items-center justify-between">
          {title && <h2 className="text-[11px] font-semibold uppercase tracking-[0.14em] text-muted">{title}</h2>}
          {right}
        </div>
      )}
      {children}
    </section>
  );
}

export function TierBadge({ tier }: { tier: Tier }) {
  return (
    <span className={`inline-flex items-center gap-1.5 rounded-full border px-2.5 py-0.5 text-xs font-medium ${tierClass[tier]}`}>
      <span className="h-1.5 w-1.5 rounded-full bg-current" />
      {tierLabel[tier]}
    </span>
  );
}

export function ErrorBox({ error, onRetry }: { error: AppError; onRetry?: () => void }) {
  return (
    <div className="rounded-xl border border-bad/30 bg-bad/5 p-4 text-sm">
      <p className="font-medium text-bad">Something went wrong</p>
      <p className="mt-1 text-fg/80">{error.message}</p>
      {onRetry && (
        <button onClick={onRetry} className="mt-3 rounded-lg border border-line px-3 py-1.5 text-xs hover:border-accent/50 hover:text-accent">
          Try again
        </button>
      )}
    </div>
  );
}

export function Loading({ label = "Loading…" }: { label?: string }) {
  return (
    <div className="flex items-center gap-3 p-4 text-sm text-muted">
      <span className="h-4 w-4 animate-spin rounded-full border-2 border-line border-t-accent" />
      {label}
    </div>
  );
}

export function Meter({ used, total }: { used: number; total: number }) {
  const pct = total > 0 ? Math.min(100, (used / total) * 100) : 0;
  return (
    <div className="h-1.5 w-full overflow-hidden rounded-full bg-panel-2">
      <div className="h-full rounded-full bg-gradient-to-r from-accent to-accent-2 transition-[width] duration-500" style={{ width: `${pct}%` }} />
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
    primary: "bg-accent text-bg hover:brightness-110 shadow-[0_0_24px_-8px_#22d3ee]",
    ghost: "border border-line text-fg hover:border-accent/50 hover:text-accent",
    danger: "border border-bad/30 text-bad hover:bg-bad/10",
  }[variant];
  return (
    <button
      title={title}
      disabled={disabled}
      onClick={onClick}
      className={`rounded-lg px-3.5 py-1.5 text-sm font-medium transition disabled:cursor-not-allowed disabled:opacity-40 disabled:shadow-none ${styles}`}
    >
      {children}
    </button>
  );
}

/** Simple on/off switch. */
export function Toggle({ on, onChange, label }: { on: boolean; onChange: (v: boolean) => void; label: string }) {
  return (
    <button
      role="switch"
      aria-checked={on}
      onClick={() => onChange(!on)}
      className="flex w-full items-center justify-between gap-3 rounded-lg px-2 py-1.5 text-xs text-muted hover:text-fg"
    >
      {label}
      <span className={`relative h-4 w-7 rounded-full transition ${on ? "bg-accent" : "bg-line"}`}>
        <span className={`absolute top-0.5 h-3 w-3 rounded-full bg-bg transition-all ${on ? "left-3.5" : "left-0.5"}`} />
      </span>
    </button>
  );
}

/** Brand mark: cyan ring + yellow core. */
export function Logo({ size = 28 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 32 32" aria-hidden>
      <defs>
        <linearGradient id="lg" x1="0" y1="0" x2="1" y2="1">
          <stop offset="0" stopColor="#22d3ee" />
          <stop offset="1" stopColor="#0ea5b7" />
        </linearGradient>
      </defs>
      <rect x="2" y="2" width="28" height="28" rx="9" fill="none" stroke="url(#lg)" strokeWidth="2.5" />
      <path d="M10 21 L16 9 L22 21" fill="none" stroke="#22d3ee" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round" />
      <circle cx="16" cy="19" r="2.6" fill="#facc15" />
    </svg>
  );
}
