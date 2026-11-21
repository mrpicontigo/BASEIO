import { useMemo, useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { api } from "../lib/api";
import { formatBytes, formatParams, useCaseLabel } from "../lib/format";
import { useAsync } from "../lib/useAsync";
import { Button, Card, ErrorBox, Loading, TierBadge } from "../components/ui";
import { InstallDialog } from "../components/InstallDialog";
import { AskBox } from "../components/AskBox";
import { isActive, useInstalls } from "../lib/installs";
import type { CatalogModel, Compatibility, RuntimeStatus } from "../lib/types";

const USE_CASES = ["chat", "writing", "coding", "reasoning", "vision", "multilingual"];

interface RowProps {
  v: CatalogModel["variants"][number]; c?: Compatibility; license: CatalogModel["license"];
  installed: boolean; onInstall: (tag: string) => void;
}

function VariantRow({ v, c, license, installed, onInstall }: RowProps) {
  const lic = v.license ?? license;
  const { installs } = useInstalls();
  const downloading = isActive(installs[v.tag]);
  return (
    <div className="flex flex-wrap items-center gap-3 border-t border-line py-2 text-sm first:border-0">
      <code className="min-w-40 font-mono text-fg">{v.tag}</code>
      <span className="text-muted">{formatParams(v.params_b)} · {v.quantization}</span>
      <span className="text-muted">~{formatBytes(v.approx_size_bytes)} download</span>
      {c && <span className="text-muted">needs ~{formatBytes(c.estimated_memory_bytes)} memory</span>}
      <span className="ml-auto flex items-center gap-2">
        {lic.gated && <span className="text-xs text-slow">Gated licence</span>}
        {c && <span title={c.reasons.join("\n")}><TierBadge tier={c.tier} /></span>}
        {installed ? <span className="w-24 text-center text-xs text-great">Installed ✓</span>
          : downloading ? <span className="w-24 text-center text-xs text-muted">Downloading…</span>
          : <Button variant="ghost" onClick={() => onInstall(v.tag)}>Install</Button>}
      </span>
    </div>
  );
}

export default function DiscoverPage({ runtime }: { runtime: RuntimeStatus | null }) {
  const [useCase, setUseCase] = useState<string | null>(null);
  const [installing, setInstalling] = useState<string | null>(null);
  const { version } = useInstalls();
  const running = runtime?.running ?? false;
  const installedList = useAsync(() => (running ? api.listInstalled() : Promise.resolve([])), [running, version]);
  const installedSet = useMemo(() => new Set((installedList.data ?? []).map((m) => m.name)), [installedList.data]);
  const catalog = useAsync(api.catalog);
  const assessment = useAsync(api.assessCatalog);
  const recs = useAsync(() => api.recommendations(useCase ?? undefined), [useCase]);

  const byTag = useMemo(() => {
    const m = new Map<string, Compatibility>();
    assessment.data?.variants.forEach((c) => m.set(c.tag, c));
    return m;
  }, [assessment.data]);

  const models = (catalog.data?.models ?? []).filter((m) => !useCase || m.use_cases.includes(useCase));

  return (
    <div className="space-y-4">
      {installing && <InstallDialog tag={installing} onClose={() => setInstalling(null)} />}
      <header>
        <h1 className="text-xl font-semibold">Discover models</h1>
        <p className="text-sm text-muted">
          Ratings are estimates based on your hardware
          {assessment.data?.disk_free_bytes != null && <> · {formatBytes(assessment.data.disk_free_bytes)} free where models are stored</>}.
        </p>
      </header>

      <AskBox installed={installedSet} onInstall={setInstalling} />

      <div className="flex flex-wrap gap-2">
        {[null, ...USE_CASES].map((u) => (
          <button
            key={u ?? "all"}
            onClick={() => setUseCase(u)}
            className={`rounded-full border px-3 py-1 text-sm ${useCase === u ? "border-accent bg-accent/20 text-fg" : "border-line text-muted hover:text-fg"}`}
          >
            {u ? useCaseLabel[u] : "All"}
          </button>
        ))}
      </div>

      <Card title="Best for your computer">
        {recs.error && <ErrorBox error={recs.error} onRetry={recs.reload} />}
        {recs.loading && <Loading label="Checking what fits…" />}
        {!recs.loading && recs.data?.length === 0 && (
          <p className="text-sm text-muted">No model in this category runs well on this computer.</p>
        )}
        <div className="grid gap-2 md:grid-cols-2">
          {recs.data?.slice(0, 4).map((r) => (
            <div key={r.variant.tag} className="rounded border border-line bg-panel-2 p-3">
              <div className="flex items-center justify-between">
                <span className="font-medium">{r.model_name}</span>
                <TierBadge tier={r.compatibility.tier} />
              </div>
              <p className="mt-1 font-mono text-xs text-muted">{r.variant.tag} · ~{formatBytes(r.variant.approx_size_bytes)}</p>
              <p className="mt-1 text-xs text-fg/80">{r.compatibility.reasons[0]}</p>
              <div className="mt-2">
                {installedSet.has(r.variant.tag)
                  ? <span className="text-xs text-great">Installed ✓</span>
                  : <Button onClick={() => setInstalling(r.variant.tag)}>Install</Button>}
              </div>
            </div>
          ))}
        </div>
      </Card>

      {(catalog.error || assessment.error) && (
        <ErrorBox error={(catalog.error ?? assessment.error)!} onRetry={() => { catalog.reload(); assessment.reload(); }} />
      )}
      {catalog.loading && <Loading />}

      {models.map((m) => (
        <Card
          key={m.id}
          right={
            <button onClick={() => openUrl(m.homepage)} className="text-xs text-accent hover:underline">
              Model page ↗
            </button>
          }
        >
          <div className="mb-2">
            <h3 className="text-base font-semibold">{m.name} <span className="text-sm font-normal text-muted">by {m.publisher}</span></h3>
            <p className="text-sm text-fg/80">{m.description}</p>
            <div className="mt-2 flex flex-wrap gap-1.5 text-xs">
              {m.use_cases.map((u) => (
                <span key={u} className="rounded bg-panel-2 px-2 py-0.5 text-muted">{useCaseLabel[u] ?? u}</span>
              ))}
              <button onClick={() => openUrl(m.license.url)} className="rounded bg-panel-2 px-2 py-0.5 text-muted hover:text-fg">
                Licence: {m.license.name}{m.license.commercial_use === "conditional" ? " (conditions apply)" : ""}
              </button>
            </div>
          </div>
          {m.variants.map((v) => (
            <VariantRow key={v.tag} v={v} c={byTag.get(v.tag)} license={m.license} installed={installedSet.has(v.tag)} onInstall={setInstalling} />
          ))}
        </Card>
      ))}
    </div>
  );
}
