import { useEffect, useState } from "react";
import { api } from "../lib/api";
import { formatBytes } from "../lib/format";
import { isActive, useInstalls } from "../lib/installs";
import { useAsync } from "../lib/useAsync";
import { Button, Card, ErrorBox, Loading, Meter } from "../components/ui";
import type { AppError, RuntimeStatus } from "../lib/types";

function Downloads() {
  const { installs, cancel, dismiss, start } = useInstalls();
  const tags = Object.keys(installs);
  if (tags.length === 0) return null;
  return (
    <Card title="Downloads">
      {tags.map((tag) => {
        const s = installs[tag];
        const pct = s.total ? Math.floor((s.completed / s.total) * 100) : 0;
        return (
          <div key={tag} className="border-t border-line py-3 first:border-0 first:pt-0">
            <div className="mb-1 flex items-center justify-between gap-3 text-sm">
              <code className="font-mono">{tag}</code>
              <span className={s.phase === "error" ? "text-bad" : s.phase === "done" ? "text-great" : "text-muted"}>
                {s.phase === "downloading" ? `${pct}% · ${formatBytes(s.completed)} / ${formatBytes(s.total)}` : s.message}
              </span>
            </div>
            {s.phase === "downloading" && <Meter used={s.completed} total={s.total} />}
            <div className="mt-2 flex gap-2">
              {isActive(s) && <Button variant="ghost" onClick={() => cancel(tag)}>Cancel</Button>}
              {(s.phase === "error" || s.phase === "cancelled") && <Button variant="ghost" onClick={() => start(tag, false)}>Retry</Button>}
              {!isActive(s) && <Button variant="ghost" onClick={() => dismiss(tag)}>Dismiss</Button>}
            </div>
          </div>
        );
      })}
    </Card>
  );
}

export default function ModelsPage({ runtime, onChat }: { runtime: RuntimeStatus | null; onChat: (model: string) => void }) {
  const { version } = useInstalls();
  const running = runtime?.running ?? false;
  const installed = useAsync(() => (running ? api.listInstalled() : Promise.resolve([])), [running, version]);
  const loaded = useAsync(() => (running ? api.listRunning() : Promise.resolve([])), [running, version]);
  const [busy, setBusy] = useState<string | null>(null);
  const [actionErr, setActionErr] = useState<AppError | null>(null);

  // Keep "in memory" status fresh while the page is open.
  useEffect(() => {
    if (!running) return;
    const t = setInterval(() => void loaded.reload(), 5000);
    return () => clearInterval(t);
  }, [running, loaded.reload]);

  const act = async (model: string, fn: () => Promise<void>) => {
    setBusy(model);
    setActionErr(null);
    try { await fn(); } catch (e) { setActionErr(e as AppError); } finally {
      setBusy(null);
      void installed.reload();
      void loaded.reload();
    }
  };

  const remove = (model: string) => {
    if (!window.confirm(`Delete ${model}? This frees its disk space. You can download it again later.`)) return;
    void act(model, () => api.deleteModel(model));
  };

  return (
    <div className="space-y-4">
      <header>
        <h1 className="text-xl font-semibold">My models</h1>
        <p className="text-sm text-muted">
          {runtime?.running ? `Ollama ${runtime.version} is running.` : "Start the AI engine to manage models."}
        </p>
      </header>

      <Downloads />
      {actionErr && <ErrorBox error={actionErr} />}

      {running && (
        <Card title="Installed">
          {installed.loading && !installed.data && <Loading />}
          {installed.error && <ErrorBox error={installed.error} onRetry={installed.reload} />}
          {installed.data?.length === 0 && <p className="text-sm text-muted">No models yet. Find one on the Discover page.</p>}
          {installed.data?.map((m) => {
            const live = loaded.data?.find((r) => r.name === m.name);
            const gpuPct = live && live.size ? Math.round((live.size_vram / live.size) * 100) : 0;
            return (
              <div key={m.name} className="flex flex-wrap items-center gap-3 border-t border-line py-3 first:border-0">
                <div className="min-w-48">
                  <code className="font-mono text-sm">{m.name}</code>
                  <p className="text-xs text-muted">
                    {formatBytes(m.size)}
                    {m.details?.parameter_size && ` · ${m.details.parameter_size}`}
                    {m.details?.quantization_level && ` · ${m.details.quantization_level}`}
                  </p>
                </div>
                <span className="text-xs">
                  {live ? <span className="text-great">In memory · {gpuPct}% on GPU</span> : <span className="text-muted">Not loaded</span>}
                </span>
                <span className="ml-auto flex gap-2">
                  <Button onClick={() => onChat(m.name)}>Chat</Button>
                  {live
                    ? <Button variant="ghost" disabled={busy === m.name} onClick={() => act(m.name, () => api.unloadModel(m.name))}>Unload</Button>
                    : <Button variant="ghost" disabled={busy === m.name} onClick={() => act(m.name, () => api.loadModel(m.name))}>{busy === m.name ? "Loading…" : "Load"}</Button>}
                  <Button variant="danger" disabled={busy === m.name} onClick={() => remove(m.name)}>Delete</Button>
                </span>
              </div>
            );
          })}
        </Card>
      )}
    </div>
  );
}
