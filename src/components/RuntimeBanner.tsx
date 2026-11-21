import { useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { api } from "../lib/api";
import type { AppError, RuntimeStatus } from "../lib/types";
import { Button } from "./ui";

/** Shown when Ollama is missing or stopped; offers the fix. */
export function RuntimeBanner({ status, onChange }: { status: RuntimeStatus | null; onChange: () => void }) {
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<AppError | null>(null);
  if (!status || status.running) return null;

  const start = async () => {
    setBusy(true);
    setErr(null);
    try { await api.startRuntime(); onChange(); } catch (e) { setErr(e as AppError); } finally { setBusy(false); }
  };

  return (
    <div className="mb-4 rounded-lg border border-slow/40 bg-slow/10 p-4 text-sm">
      {status.installed ? (
        <div className="flex items-center justify-between gap-4">
          <div>
            <p className="font-medium text-slow">The AI engine (Ollama) is not running</p>
            <p className="text-fg/80">Start it to download and chat with models.</p>
          </div>
          <Button onClick={start} disabled={busy}>{busy ? "Starting…" : "Start engine"}</Button>
        </div>
      ) : (
        <div className="flex items-center justify-between gap-4">
          <div>
            <p className="font-medium text-slow">The AI engine (Ollama) is not installed</p>
            <p className="text-fg/80">BASEIO uses Ollama to run models. Install it, then press “Check again”.</p>
          </div>
          <div className="flex gap-2">
            <Button onClick={() => openUrl("https://ollama.com/download/windows")}>Get Ollama ↗</Button>
            <Button variant="ghost" onClick={onChange}>Check again</Button>
          </div>
        </div>
      )}
      {err && <p className="mt-2 text-bad">{err.message}</p>}
    </div>
  );
}
