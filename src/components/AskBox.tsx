import { useState } from "react";
import { api } from "../lib/api";
import { formatBytes } from "../lib/format";
import type { AppError, RequestInterpretation } from "../lib/types";
import { Button, ErrorBox, TierBadge } from "./ui";

const EXAMPLES = [
  "A fast model to help me with Python on my laptop",
  "The best model for writing emails for my business",
  "Something that can describe photos, under 5 GB",
];

/** Plain-language request → recommendations. Installing still needs approval. */
export function AskBox({ installed, onInstall }: { installed: Set<string>; onInstall: (tag: string) => void }) {
  const [text, setText] = useState("");
  const [busy, setBusy] = useState(false);
  const [res, setRes] = useState<RequestInterpretation | null>(null);
  const [err, setErr] = useState<AppError | null>(null);

  const ask = async (q = text) => {
    if (!q.trim()) return;
    setText(q);
    setBusy(true);
    setErr(null);
    try { setRes(await api.interpretRequest(q)); } catch (e) { setErr(e as AppError); } finally { setBusy(false); }
  };

  return (
    <section className="rounded-lg border border-accent/40 bg-accent/5 p-4">
      <p className="mb-2 text-sm font-medium">What do you want to use AI for?</p>
      <div className="flex gap-2">
        <input
          className="flex-1 rounded border border-line bg-panel-2 px-3 py-2 text-sm outline-none focus:border-accent"
          placeholder="e.g. help me write code on my laptop"
          value={text}
          maxLength={2000}
          onChange={(e) => setText(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && void ask()}
        />
        <Button onClick={() => ask()} disabled={busy || !text.trim()}>{busy ? "Thinking…" : "Find models"}</Button>
      </div>
      {!res && (
        <div className="mt-2 flex flex-wrap gap-2">
          {EXAMPLES.map((ex) => (
            <button key={ex} onClick={() => ask(ex)} className="rounded-full border border-line px-3 py-1 text-xs text-muted hover:text-fg">
              {ex}
            </button>
          ))}
        </div>
      )}
      {err && <div className="mt-3"><ErrorBox error={err} /></div>}
      {res && (
        <div className="mt-3 space-y-2">
          <div className="flex flex-wrap gap-1.5 text-xs">
            {res.intent.understood.length === 0
              ? <span className="text-muted">No specific needs recognised, showing general models.</span>
              : res.intent.understood.map((u) => <span key={u} className="rounded bg-panel-2 px-2 py-0.5 text-muted">{u}</span>)}
          </div>
          {res.recommendations.length === 0 && (
            <p className="text-sm text-muted">No model in the catalogue matches all of that and runs well here. Try loosening a requirement.</p>
          )}
          <div className="grid gap-2 md:grid-cols-3">
            {res.recommendations.slice(0, 3).map((r) => (
              <div key={r.variant.tag} className="rounded border border-line bg-panel p-3">
                <div className="flex items-center justify-between gap-2">
                  <span className="font-medium">{r.model_name}</span>
                  <TierBadge tier={r.compatibility.tier} />
                </div>
                <p className="mt-1 font-mono text-xs text-muted">{r.variant.tag} · ~{formatBytes(r.variant.approx_size_bytes)}</p>
                <div className="mt-2">
                  {installed.has(r.variant.tag)
                    ? <span className="text-xs text-great">Installed ✓</span>
                    : <Button onClick={() => onInstall(r.variant.tag)}>Install</Button>}
                </div>
              </div>
            ))}
          </div>
        </div>
      )}
    </section>
  );
}
