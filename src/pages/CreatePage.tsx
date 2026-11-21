import { useEffect, useMemo, useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { save } from "@tauri-apps/plugin-dialog";
import { api } from "../lib/api";
import { formatBytes } from "../lib/format";
import { isGenerating, useImageJobs } from "../lib/imageJobs";
import { useAsync } from "../lib/useAsync";
import { Button, Card, ErrorBox, Loading, Meter, TierBadge } from "../components/ui";
import type { AppError, GeneratedImage, ImageModelStatus } from "../lib/types";

const IDEAS = [
  "A cosy reading nook with a stack of books and a sleeping dog, warm evening light",
  "A small rocket launching over snowy mountains at sunrise, digital art",
  "A wooden sailing boat on a calm turquoise sea, watercolour",
  "A friendly goat wearing sunglasses on a hill, cartoon style",
];

function useImageSrc(id: number | null) {
  const [src, setSrc] = useState<string | null>(null);
  useEffect(() => {
    let alive = true;
    setSrc(null);
    if (id != null) api.imageData(id).then((b) => alive && setSrc(`data:image/png;base64,${b}`)).catch(() => {});
    return () => { alive = false; };
  }, [id]);
  return src;
}

function Thumb({ img, onOpen }: { img: GeneratedImage; onOpen: () => void }) {
  const src = useImageSrc(img.id);
  return (
    <button onClick={onOpen} className="group relative aspect-square overflow-hidden rounded-xl border border-line bg-panel-2 transition hover:border-accent/50" title={img.prompt}>
      {src ? <img src={src} alt={img.prompt} className="h-full w-full object-cover transition group-hover:scale-[1.03]" /> : <span className="text-xs text-muted">…</span>}
    </button>
  );
}

function Viewer({ img, onClose, onDeleted, onReuse }: { img: GeneratedImage; onClose: () => void; onDeleted: () => void; onReuse: (p: string) => void }) {
  const src = useImageSrc(img.id);
  const [err, setErr] = useState<AppError | null>(null);
  const saveAs = async () => {
    try {
      const dest = await save({ defaultPath: `baseio-${img.id}.png`, filters: [{ name: "PNG image", extensions: ["png"] }] });
      if (dest) await api.exportImage(img.id, dest);
    } catch (e) { setErr(e as AppError); }
  };
  const del = async () => {
    if (!window.confirm("Delete this image?")) return;
    try { await api.deleteGeneratedImage(img.id); onDeleted(); } catch (e) { setErr(e as AppError); }
  };
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/80 p-6 backdrop-blur-sm" onClick={onClose}>
      <div className="flex max-h-full w-full max-w-5xl gap-5 rounded-2xl border border-line bg-panel-2 p-4" onClick={(e) => e.stopPropagation()}>
        <div className="flex min-w-0 flex-1 items-center justify-center">
          {src ? <img src={src} alt={img.prompt} className="max-h-[80vh] rounded-xl object-contain" /> : <Loading />}
        </div>
        <div className="flex w-64 shrink-0 flex-col gap-3 text-sm">
          <p className="text-xs uppercase tracking-wider text-muted">Prompt</p>
          <p className="max-h-48 overflow-y-auto text-fg/90">{img.prompt}</p>
          <p className="text-xs text-muted">
            {img.width}×{img.height} · {img.steps} steps · seed {img.seed}<br />
            {img.model_id} · took {(img.duration_ms / 1000).toFixed(0)} s
          </p>
          {err && <ErrorBox error={err} />}
          <div className="mt-auto flex flex-col gap-2">
            <Button onClick={saveAs}>Save as…</Button>
            <Button variant="ghost" onClick={() => onReuse(img.prompt)}>Use this prompt</Button>
            <Button variant="danger" onClick={del}>Delete</Button>
          </div>
        </div>
      </div>
    </div>
  );
}

function InstallCard({ s, engineBytes, onInstalled }: { s: ImageModelStatus; engineBytes: number; onInstalled: () => void }) {
  const { installs, install, cancelInstall } = useImageJobs();
  const job = installs[s.model.id];
  const [accepted, setAccepted] = useState(false);
  const [prev, setPrev] = useState(false);
  useEffect(() => { api.imageLicenseAccepted(s.model.id).then(setPrev).catch(() => {}); }, [s.model.id]);
  useEffect(() => { if (job?.phase === "done") onInstalled(); }, [job?.phase]); // eslint-disable-line react-hooks/exhaustive-deps
  const total = s.download_bytes + engineBytes;
  const blocked = s.fit.tier === "too_large";
  return (
    <div className="rounded-xl border border-line bg-panel-2/60 p-4">
      <div className="flex items-start justify-between gap-3">
        <div>
          <p className="font-semibold">
            {s.model.name} <span className="text-xs font-normal text-muted">by {s.model.publisher}</span>
            {!s.model.verified && <span title="This model has not yet been test-run by the BASEIO team. It should work but may fail on some PCs." className="ml-2 rounded-full border border-accent-2/40 px-2 py-0.5 text-[10px] font-normal text-accent-2">Not yet tested</span>}
          </p>
          <p className="mt-1 text-sm text-fg/80">{s.model.description}</p>
        </div>
        <TierBadge tier={s.fit.tier} />
      </div>
      <p className="mt-2 text-xs text-muted">{s.fit.reasons.join(" · ")}</p>
      <p className="mt-1 text-xs text-muted">
        Download {formatBytes(total)}{engineBytes > 0 && ` (includes the ${formatBytes(engineBytes)} image engine)`} · verified with sha256
      </p>
      <div className="mt-2 text-xs">
        <button className="text-accent hover:underline" onClick={() => openUrl(s.model.license.url)}>Licence: {s.model.license.name} ↗</button>
        {s.model.license.commercial_use === "conditional" && <span className="ml-2 text-accent-2">conditions apply</span>}
      </div>
      {job?.phase === "running" ? (
        <div className="mt-3 space-y-2">
          <div className="flex justify-between text-xs text-muted">
            <span>{job.message}</span>
            {job.total > 0 && <span>{formatBytes(job.completed)} / {formatBytes(job.total)}</span>}
          </div>
          <Meter used={job.completed} total={job.total || 1} />
          <Button variant="ghost" onClick={() => cancelInstall(s.model.id)}>Cancel</Button>
        </div>
      ) : (
        <div className="mt-3 flex flex-wrap items-center gap-3">
          {!prev && (
            <label className="flex items-center gap-2 text-sm">
              <input type="checkbox" checked={accepted} onChange={(e) => setAccepted(e.target.checked)} />
              I accept the licence
            </label>
          )}
          <Button disabled={blocked || (!prev && !accepted)} onClick={() => install(s.model.id, accepted)}>
            {job?.phase === "cancelled" || job?.phase === "error" ? "Retry install" : "Install"}
          </Button>
          {job && job.phase !== "done" && <span className={`text-xs ${job.phase === "error" ? "text-bad" : "text-muted"}`}>{job.message}</span>}
        </div>
      )}
    </div>
  );
}

export default function CreatePage() {
  const { gen, generate, cancelGenerate, version } = useImageJobs();
  const overview = useAsync(api.imageOverview, [version]);
  const gallery = useAsync(api.listGeneratedImages, [version]);
  const installed = useMemo(() => (overview.data?.models ?? []).filter((m) => m.installed), [overview.data]);
  const [modelId, setModelId] = useState("");
  const [prompt, setPrompt] = useState("");
  const [negative, setNegative] = useState("");
  const [showNeg, setShowNeg] = useState(false);
  const [size, setSize] = useState<[number, number] | null>(null);
  const [viewing, setViewing] = useState<GeneratedImage | null>(null);
  const [showModels, setShowModels] = useState(false);
  const [now, setNow] = useState(Date.now());
  const busy = isGenerating(gen);

  const current = installed.find((m) => m.model.id === modelId) ?? installed[0];
  useEffect(() => { if (current && current.model.id !== modelId) setModelId(current.model.id); }, [current, modelId]);
  useEffect(() => { if (current) setSize([current.model.defaults.width, current.model.defaults.height]); }, [current?.model.id]); // eslint-disable-line react-hooks/exhaustive-deps
  useEffect(() => {
    if (!busy) return;
    const t = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(t);
  }, [busy]);

  const go = () => {
    if (!current || !size || !prompt.trim()) return;
    void generate({ model_id: current.model.id, prompt, negative_prompt: negative || null, width: size[0], height: size[1] });
  };

  const latestSrcId = gen?.result?.id ?? null;
  const latestSrc = useImageSrc(latestSrcId);
  const pct = gen && gen.total ? Math.round((gen.step / gen.total) * 100) : 0;

  return (
    <div className="space-y-5">
      {viewing && (
        <Viewer
          img={viewing}
          onClose={() => setViewing(null)}
          onDeleted={() => { setViewing(null); void gallery.reload(); }}
          onReuse={(p) => { setPrompt(p); setViewing(null); }}
        />
      )}
      <header className="flex items-end justify-between">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">Create images</h1>
          <p className="text-sm text-muted">Generated entirely on this computer. Nothing is uploaded.</p>
        </div>
        {installed.length > 0 && <Button variant="ghost" onClick={() => setShowModels((s) => !s)}>{showModels ? "Hide models" : "Image models"}</Button>}
      </header>

      {overview.loading && !overview.data && <Loading />}
      {overview.error && <ErrorBox error={overview.error} onRetry={overview.reload} />}

      {overview.data && (installed.length === 0 || showModels) && (
        <Card title={installed.length === 0 ? "Install an image model to start" : "Image models"}>
          <div className="grid gap-3 md:grid-cols-2">
            {overview.data.models.map((s) =>
              s.installed ? (
                <div key={s.model.id} className="rounded-xl border border-line bg-panel-2/60 p-4">
                  <p className="font-semibold">{s.model.name} <span className="text-xs text-great">Installed ✓</span></p>
                  <p className="mt-1 text-xs text-muted">{s.fit.reasons.join(" · ")}</p>
                  <div className="mt-3">
                    <Button variant="danger" onClick={async () => {
                      if (!window.confirm(`Delete ${s.model.name}? Your generated images are kept.`)) return;
                      try { await api.deleteImageModel(s.model.id); } finally { void overview.reload(); }
                    }}>Delete model</Button>
                  </div>
                </div>
              ) : (
                <InstallCard key={s.model.id} s={s} engineBytes={overview.data!.engine_download_bytes} onInstalled={overview.reload} />
              ),
            )}
          </div>
        </Card>
      )}

      {current && (
        <section className="rounded-2xl border border-accent-2/25 bg-gradient-to-br from-accent/10 via-panel to-accent-2/10 p-5 backdrop-blur-md">
          <textarea
            value={prompt}
            onChange={(e) => setPrompt(e.target.value)}
            onKeyDown={(e) => { if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) go(); }}
            maxLength={1500}
            rows={3}
            placeholder="Describe the image you want… (Ctrl+Enter to create)"
            className="w-full resize-none rounded-xl border border-line bg-bg/60 px-4 py-3 text-[15px] outline-none transition focus:border-accent/60"
          />
          {!prompt && (
            <div className="mt-2 flex flex-wrap gap-2">
              {IDEAS.map((i) => (
                <button key={i} onClick={() => setPrompt(i)} className="rounded-full border border-line px-3 py-1 text-xs text-muted transition hover:border-accent-2/50 hover:text-accent-2">{i}</button>
              ))}
            </div>
          )}
          {showNeg && (
            <input
              value={negative}
              onChange={(e) => setNegative(e.target.value)}
              maxLength={800}
              placeholder="Things to avoid, e.g. blurry, text, extra fingers"
              className="mt-2 w-full rounded-xl border border-line bg-bg/60 px-4 py-2 text-sm outline-none focus:border-accent/60"
            />
          )}
          <div className="mt-3 flex flex-wrap items-center gap-2">
            {installed.length > 1 && (
              <select
                value={current.model.id}
                onChange={(e) => setModelId(e.target.value)}
                className="rounded-lg border border-line bg-panel-2 px-2 py-1.5 text-sm outline-none"
              >
                {installed.map((m) => <option key={m.model.id} value={m.model.id}>{m.model.name}</option>)}
              </select>
            )}
            {installed.length === 1 && <span className="text-sm text-muted">{current.model.name}</span>}
            <span className="mx-1 h-4 w-px bg-line" />
            {current.model.sizes.map(([w, h]) => (
              <button
                key={`${w}x${h}`}
                onClick={() => setSize([w, h])}
                className={`rounded-full border px-3 py-1 text-xs transition ${size?.[0] === w && size?.[1] === h ? "border-accent/60 bg-accent/10 text-accent" : "border-line text-muted hover:text-fg"}`}
              >{w === h ? "Square" : w > h ? "Wide" : "Tall"} {w}×{h}</button>
            ))}
            <button onClick={() => setShowNeg((s) => !s)} className="text-xs text-muted hover:text-fg">{showNeg ? "− Avoid" : "+ Avoid"}</button>
            <span className="ml-auto" />
            {busy ? <Button variant="ghost" onClick={cancelGenerate}>Stop</Button> : <Button onClick={go} disabled={!prompt.trim()}>Create</Button>}
          </div>
          <p className="mt-2 text-xs text-muted">{current.fit.reasons.join(" · ")}</p>
        </section>
      )}

      {gen && (
        <Card title="Latest">
          <div className="flex flex-col items-center gap-3">
            {latestSrc ? (
              <button onClick={() => gen.result && setViewing(gen.result)}>
                <img src={latestSrc} alt={gen.request.prompt} className="max-h-[420px] rounded-xl border border-line" />
              </button>
            ) : busy ? (
              <div className="flex aspect-square w-full max-w-sm flex-col items-center justify-center gap-3 rounded-xl border border-dashed border-line">
                <span className="h-8 w-8 animate-spin rounded-full border-2 border-line border-t-accent" />
                <p className="text-sm">{gen.status}</p>
                {gen.total > 0 && <div className="w-48"><Meter used={gen.step} total={gen.total} /></div>}
                <p className="text-xs text-muted">{gen.total > 0 && `Step ${gen.step}/${gen.total} · ${pct}% · `}{Math.round((now - gen.startedAt) / 1000)} s</p>
              </div>
            ) : null}
            {gen.error && <ErrorBox error={gen.error} />}
            {gen.cancelled && <p className="text-sm text-muted">Stopped.</p>}
          </div>
        </Card>
      )}

      {(gallery.data?.length ?? 0) > 0 && (
        <Card title="Your images">
          <div className="grid grid-cols-3 gap-3 md:grid-cols-4 lg:grid-cols-5">
            {gallery.data!.map((img) => <Thumb key={img.id} img={img} onOpen={() => setViewing(img)} />)}
          </div>
        </Card>
      )}
    </div>
  );
}
