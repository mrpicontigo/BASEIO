import { createContext, useCallback, useContext, useState, type ReactNode } from "react";
import { api } from "./api";
import type { AppError } from "./types";

export interface InstallState {
  phase: "starting" | "downloading" | "verifying" | "done" | "error" | "cancelled";
  message: string;
  completed: number;
  total: number;
  error?: AppError;
}

interface Ctx {
  installs: Record<string, InstallState>;
  start: (tag: string, acceptLicense: boolean) => Promise<void>;
  cancel: (tag: string) => Promise<void>;
  dismiss: (tag: string) => void;
  /** Incremented after every successful install so pages can refresh. */
  version: number;
}

const InstallCtx = createContext<Ctx | null>(null);

/** App-level download manager: progress survives page navigation. */
export function InstallProvider({ children }: { children: ReactNode }) {
  const [installs, setInstalls] = useState<Record<string, InstallState>>({});
  const [version, setVersion] = useState(0);

  const patch = useCallback((tag: string, p: Partial<InstallState>) => {
    setInstalls((s) => ({ ...s, [tag]: { ...(s[tag] ?? { phase: "starting", message: "", completed: 0, total: 0 }), ...p } }));
  }, []);

  const start = useCallback(async (tag: string, acceptLicense: boolean) => {
    patch(tag, { phase: "starting", message: "Starting download…", completed: 0, total: 0, error: undefined });
    try {
      await api.installModel(tag, acceptLicense, (e) => {
        switch (e.event) {
          case "status": patch(tag, { message: e.message }); break;
          case "progress": patch(tag, { phase: "downloading", completed: e.completed, total: e.total, message: "Downloading" }); break;
          case "verifying": patch(tag, { phase: "verifying", message: "Verifying installation…" }); break;
          case "done": patch(tag, { phase: "done", message: "Installed" }); break;
        }
      });
      patch(tag, { phase: "done", message: "Installed" });
      setVersion((v) => v + 1);
    } catch (e) {
      const err = e as AppError;
      if (err.kind === "cancelled") patch(tag, { phase: "cancelled", message: "Download cancelled. Partial data is kept so it can resume." });
      else patch(tag, { phase: "error", message: err.message, error: err });
    }
  }, [patch]);

  const cancel = useCallback(async (tag: string) => { await api.cancelInstall(tag); }, []);
  const dismiss = useCallback((tag: string) => {
    setInstalls((s) => { const n = { ...s }; delete n[tag]; return n; });
  }, []);

  return <InstallCtx.Provider value={{ installs, start, cancel, dismiss, version }}>{children}</InstallCtx.Provider>;
}

export function useInstalls(): Ctx {
  const c = useContext(InstallCtx);
  if (!c) throw new Error("useInstalls outside InstallProvider");
  return c;
}

export function isActive(s?: InstallState) {
  return !!s && (s.phase === "starting" || s.phase === "downloading" || s.phase === "verifying");
}
