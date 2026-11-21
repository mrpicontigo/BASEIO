import { createContext, useCallback, useContext, useState, type ReactNode } from "react";
import { api } from "./api";
import type { AppError, GenRequest, GeneratedImage } from "./types";

export interface ImageInstall { phase: "running" | "done" | "error" | "cancelled"; message: string; completed: number; total: number }
export interface Generation {
  request: GenRequest; startedAt: number; status: string; step: number; total: number;
  result?: GeneratedImage; error?: AppError; cancelled?: boolean;
}

interface Ctx {
  installs: Record<string, ImageInstall>;
  install: (modelId: string, accept: boolean) => Promise<void>;
  cancelInstall: (modelId: string) => void;
  gen: Generation | null;
  generate: (r: GenRequest) => Promise<void>;
  cancelGenerate: () => void;
  /** Bumped when installs or generations finish, so pages can refresh. */
  version: number;
}

const C = createContext<Ctx | null>(null);

/** App-level state so downloads and generations survive page switches. */
export function ImageJobsProvider({ children }: { children: ReactNode }) {
  const [installs, setInstalls] = useState<Record<string, ImageInstall>>({});
  const [gen, setGen] = useState<Generation | null>(null);
  const [version, setVersion] = useState(0);

  const patch = (id: string, p: Partial<ImageInstall>) =>
    setInstalls((s) => ({ ...s, [id]: { ...(s[id] ?? { phase: "running", message: "", completed: 0, total: 0 }), ...p } }));

  const install = useCallback(async (modelId: string, accept: boolean) => {
    patch(modelId, { phase: "running", message: "Starting…", completed: 0, total: 0 });
    try {
      await api.installImageModel(modelId, accept, (e) => {
        if (e.event === "status") patch(modelId, { message: e.message });
        else if (e.event === "progress") patch(modelId, { completed: e.completed, total: e.total });
      });
      patch(modelId, { phase: "done", message: "Installed" });
    } catch (e) {
      const err = e as AppError;
      patch(modelId, err.kind === "cancelled"
        ? { phase: "cancelled", message: "Cancelled. Downloaded parts are kept, so Install resumes." }
        : { phase: "error", message: err.message });
    } finally {
      setVersion((v) => v + 1);
    }
  }, []);

  const cancelInstall = useCallback((modelId: string) => { void api.cancelImageInstall(modelId); }, []);

  const generate = useCallback(async (request: GenRequest) => {
    setGen({ request, startedAt: Date.now(), status: "Starting…", step: 0, total: 0 });
    try {
      const result = await api.generateImage(request, (e) => {
        if (e.event === "status") setGen((g) => g && { ...g, status: e.message });
        else setGen((g) => g && { ...g, status: "Generating…", step: e.step, total: e.total });
      });
      setGen((g) => g && { ...g, status: "Done", result });
    } catch (e) {
      const err = e as AppError;
      setGen((g) => g && (err.kind === "cancelled" ? { ...g, cancelled: true, status: "Cancelled" } : { ...g, error: err, status: "Failed" }));
    } finally {
      setVersion((v) => v + 1);
    }
  }, []);

  const cancelGenerate = useCallback(() => { void api.cancelImageGeneration(); }, []);

  return <C.Provider value={{ installs, install, cancelInstall, gen, generate, cancelGenerate, version }}>{children}</C.Provider>;
}

export function useImageJobs(): Ctx {
  const c = useContext(C);
  if (!c) throw new Error("useImageJobs outside ImageJobsProvider");
  return c;
}

export const isGenerating = (g: Generation | null) => !!g && !g.result && !g.error && !g.cancelled;
