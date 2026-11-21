import { api } from "../lib/api";
import { formatBytes } from "../lib/format";
import { useAsync } from "../lib/useAsync";
import { Button, Card, ErrorBox, Loading, Meter } from "../components/ui";
import type { HardwareInfo } from "../lib/types";

function Row({ k, v }: { k: string; v: React.ReactNode }) {
  return (
    <div className="flex justify-between gap-4 py-1 text-sm">
      <span className="text-muted">{k}</span>
      <span className="text-right">{v}</span>
    </div>
  );
}

function Summary({ hw }: { hw: HardwareInfo }) {
  const gpu = hw.gpus.filter((g) => !g.integrated).sort((a, b) => b.dedicated_vram_bytes - a.dedicated_vram_bytes)[0];
  const ramGb = hw.memory.total_bytes / 1024 ** 3;
  let text: string;
  if (gpu && gpu.dedicated_vram_bytes >= 8 * 1024 ** 3) {
    text = `Your ${gpu.name} can run mid-sized models (7–8B) entirely on the GPU.`;
  } else if (gpu) {
    text = `Your ${gpu.name} can accelerate small models (1–4B) and speed up larger ones partially.`;
  } else if (ramGb >= 16) {
    text = "No dedicated GPU found. Small models (1–4B) will run well on your CPU; 7–8B models will run but slowly.";
  } else {
    text = "No dedicated GPU found and limited memory. Small models (1–3B) are the best fit.";
  }
  return <p className="text-sm text-fg/90">{text}</p>;
}

export default function SystemPage() {
  const { data: hw, error, loading, reload } = useAsync(api.hardware);

  return (
    <div className="space-y-4">
      <header className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">Your computer</h1>
          <p className="text-sm text-muted">Detected directly from Windows. Used to recommend models that will run well.</p>
        </div>
        <Button variant="ghost" onClick={reload} disabled={loading}>Refresh</Button>
      </header>

      {error && <ErrorBox error={error} onRetry={reload} />}
      {loading && !hw && <Loading label="Detecting hardware…" />}

      {hw && (
        <>
          <Card title="What this means"><Summary hw={hw} /></Card>

          {hw.warnings.length > 0 && (
            <div className="rounded border border-slow/40 bg-slow/10 p-3 text-sm text-slow">
              {hw.warnings.map((w) => <p key={w}>{w}</p>)}
            </div>
          )}

          <div className="grid gap-4 md:grid-cols-2">
            <Card title="Processor">
              <p className="mb-2 font-medium">{hw.cpu.brand || "Unknown CPU"}</p>
              <Row k="Cores" v={`${hw.cpu.physical_cores ?? "?"} physical / ${hw.cpu.logical_cores} logical`} />
              <Row k="Clock" v={hw.cpu.frequency_mhz ? `${(hw.cpu.frequency_mhz / 1000).toFixed(2)} GHz` : "—"} />
              <Row k="Architecture" v={hw.cpu.arch} />
              <Row
                k="AI instructions"
                v={(["avx", "avx2", "fma", "avx512f"] as const)
                  .filter((f) => hw.cpu.features[f]).map((f) => f.toUpperCase()).join(", ") || "none detected"}
              />
            </Card>

            <Card title="Memory (RAM)">
              <p className="mb-2 font-medium">{formatBytes(hw.memory.total_bytes)} total</p>
              <Meter used={hw.memory.total_bytes - hw.memory.available_bytes} total={hw.memory.total_bytes} />
              <Row k="Available now" v={formatBytes(hw.memory.available_bytes)} />
            </Card>

            <Card title="Graphics">
              {hw.gpus.length === 0 && <p className="text-sm text-muted">No hardware GPU detected.</p>}
              {hw.gpus.map((g) => (
                <div key={`${g.name}-${g.vendor_id}`} className="mb-2 border-b border-line pb-2 last:mb-0 last:border-0 last:pb-0">
                  <p className="font-medium">{g.name}</p>
                  <Row k="Type" v={g.integrated ? "Integrated" : "Dedicated"} />
                  <Row k="Video memory" v={formatBytes(g.dedicated_vram_bytes)} />
                  <Row k="Shared memory" v={formatBytes(g.shared_memory_bytes)} />
                </div>
              ))}
            </Card>

            <Card title="Storage">
              {hw.disks.map((d) => (
                <div key={d.mount_point} className="mb-3 last:mb-0">
                  <div className="mb-1 flex justify-between text-sm">
                    <span>{d.mount_point} <span className="text-muted">{d.kind.toUpperCase()}{d.removable ? " · removable" : ""}</span></span>
                    <span className="text-muted">{formatBytes(d.available_bytes)} free of {formatBytes(d.total_bytes)}</span>
                  </div>
                  <Meter used={d.total_bytes - d.available_bytes} total={d.total_bytes} />
                </div>
              ))}
            </Card>
          </div>

          <Card title="Operating system">
            <Row k="System" v={hw.os.long_version || `${hw.os.name} ${hw.os.version}`} />
            {hw.os.hostname && <Row k="Computer name" v={hw.os.hostname} />}
          </Card>
        </>
      )}
    </div>
  );
}
