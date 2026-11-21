import { useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { api } from "../lib/api";
import { formatBytes } from "../lib/format";
import { useInstalls } from "../lib/installs";
import { useAsync } from "../lib/useAsync";
import { Button, ErrorBox, Loading, TierBadge } from "./ui";

/** Approval step: shows verified size, fit, and licence before any download. */
export function InstallDialog({ tag, onClose }: { tag: string; onClose: () => void }) {
  const plan = useAsync(() => api.prepareInstall(tag), [tag]);
  const { start } = useInstalls();
  const [accepted, setAccepted] = useState(false);
  const p = plan.data;
  const licenseOk = !!p && (p.license_accepted || accepted);
  const canInstall = !!p && p.blockers.length === 0 && licenseOk;

  const install = () => {
    if (!p) return;
    void start(p.tag, accepted);
    onClose();
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4" onClick={onClose}>
      <div className="w-full max-w-lg rounded-lg border border-line bg-panel p-5" onClick={(e) => e.stopPropagation()}>
        <h2 className="text-lg font-semibold">Install {p?.model_name ?? tag}</h2>
        <p className="mb-4 font-mono text-xs text-muted">{tag}</p>

        {plan.loading && <Loading label="Checking size, compatibility and licence…" />}
        {plan.error && <ErrorBox error={plan.error} onRetry={plan.reload} />}

        {p && (
          <div className="space-y-3 text-sm">
            <div className="flex justify-between">
              <span className="text-muted">Download size</span>
              <span>
                {p.verified_size_bytes != null
                  ? <>{formatBytes(p.verified_size_bytes)} <span className="text-great">(verified)</span></>
                  : p.approx_size_bytes != null ? <>~{formatBytes(p.approx_size_bytes)} <span className="text-slow">(estimate)</span></> : "Unknown"}
              </span>
            </div>
            {p.size_error && <p className="text-xs text-slow">{p.size_error}</p>}
            <div className="flex justify-between">
              <span className="text-muted">Free disk space</span>
              <span>{formatBytes(p.disk_free_bytes)}</span>
            </div>
            {p.compatibility && (
              <div className="flex items-start justify-between gap-4">
                <span className="text-muted">On this computer</span>
                <span className="text-right">
                  <TierBadge tier={p.compatibility.tier} />
                  <span className="mt-1 block text-xs text-fg/70">{p.compatibility.reasons.join(" · ")}</span>
                </span>
              </div>
            )}

            <div className="rounded border border-line bg-panel-2 p-3">
              <p className="font-medium">Licence: {p.license.name}</p>
              {p.license.commercial_use === "conditional" && <p className="text-xs text-slow">Commercial use has conditions.</p>}
              {p.license.gated && <p className="text-xs text-slow">The publisher requires accepting terms before access.</p>}
              <button className="mt-1 text-xs text-accent hover:underline" onClick={() => openUrl(p.license.url)}>Read the licence ↗</button>
              {p.license_accepted ? (
                <p className="mt-2 text-xs text-great">You accepted this licence previously.</p>
              ) : (
                <label className="mt-2 flex items-center gap-2 text-sm">
                  <input type="checkbox" checked={accepted} onChange={(e) => setAccepted(e.target.checked)} />
                  I have read and accept the licence terms
                </label>
              )}
            </div>

            {p.blockers.length > 0 && (
              <div className="rounded border border-bad/40 bg-bad/10 p-3 text-bad">
                {p.blockers.map((b) => <p key={b}>• {b}</p>)}
              </div>
            )}
            {p.compatibility?.tier === "too_large" && p.blockers.length === 0 && (
              <p className="text-xs text-slow">This model is likely too large for this computer. You can still install it.</p>
            )}
          </div>
        )}

        <div className="mt-5 flex justify-end gap-2">
          <Button variant="ghost" onClick={onClose}>Cancel</Button>
          <Button onClick={install} disabled={!canInstall}>Download &amp; install</Button>
        </div>
      </div>
    </div>
  );
}
