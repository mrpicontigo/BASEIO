import { useEffect, useState, type ReactNode } from "react";
import SystemPage from "./pages/SystemPage";
import DiscoverPage from "./pages/DiscoverPage";
import ModelsPage from "./pages/ModelsPage";
import ChatPage from "./pages/ChatPage";
import CreatePage from "./pages/CreatePage";
import { ImageJobsProvider, isGenerating, useImageJobs } from "./lib/imageJobs";
import { RuntimeBanner } from "./components/RuntimeBanner";
import { Backdrop, useBackdropPref } from "./components/Backdrop";
import { Logo, Toggle } from "./components/ui";
import { InstallProvider, isActive, useInstalls } from "./lib/installs";
import { api } from "./lib/api";
import { useAsync } from "./lib/useAsync";

type Page = "discover" | "models" | "chat" | "create" | "system";

const icon = (d: ReactNode) => (
  <svg viewBox="0 0 24 24" className="h-[18px] w-[18px]" fill="none" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round">{d}</svg>
);

const NAV: { id: Page; label: string; icon: ReactNode }[] = [
  { id: "discover", label: "Discover", icon: icon(<><circle cx="12" cy="12" r="9" /><path d="M15.5 8.5 13.5 13.5 8.5 15.5 10.5 10.5Z" /></>) },
  { id: "models", label: "My models", icon: icon(<><rect x="3" y="4" width="18" height="6" rx="2" /><rect x="3" y="14" width="18" height="6" rx="2" /><path d="M7 7h.01M7 17h.01" /></>) },
  { id: "chat", label: "Chat", icon: icon(<path d="M20 15a2 2 0 0 1-2 2H8l-4 4V5a2 2 0 0 1 2-2h12a2 2 0 0 1 2 2Z" />) },
  { id: "create", label: "Create", icon: icon(<><rect x="3" y="3" width="18" height="18" rx="3" /><circle cx="9" cy="9" r="2" /><path d="m21 15-5-5L5 21" /></>) },
  { id: "system", label: "My PC", icon: icon(<><rect x="3" y="4" width="18" height="12" rx="2" /><path d="M8 20h8M12 16v4" /></>) },
];

function Shell() {
  const [page, setPage] = useState<Page>("discover");
  const [chatModel, setChatModel] = useState<string | null>(null);
  const [sketches, setSketches] = useBackdropPref();
  const runtime = useAsync(api.runtimeStatus);
  const { installs } = useInstalls();
  const downloading = Object.values(installs).filter(isActive).length;
  const { gen } = useImageJobs();

  // Notice if Ollama is started/stopped outside the app.
  useEffect(() => {
    const t = setInterval(() => void runtime.reload(), 15000);
    return () => clearInterval(t);
  }, [runtime.reload]);

  const rt = runtime.data;

  return (
    <div className="relative flex h-full">
      <Backdrop enabled={sketches} />

      <nav className="relative z-10 flex w-56 shrink-0 flex-col border-r border-line bg-panel backdrop-blur-xl">
        <div className="flex items-center gap-3 px-5 py-6">
          <Logo />
          <div>
            <p className="bg-gradient-to-r from-accent to-accent-2 bg-clip-text text-lg font-bold leading-none tracking-wide text-transparent">
              BASEIO
            </p>
            <p className="mt-1 text-[11px] text-muted">Local AI Hub</p>
          </div>
        </div>

        <div className="space-y-0.5 px-3">
          {NAV.map((n) => {
            const active = page === n.id;
            return (
              <button
                key={n.id}
                onClick={() => setPage(n.id)}
                className={`relative flex w-full items-center gap-3 rounded-lg px-3 py-2 text-left text-sm transition ${
                  active ? "bg-accent/10 text-accent" : "text-muted hover:bg-panel-2 hover:text-fg"
                }`}
              >
                {active && <span className="absolute -left-3 top-1.5 bottom-1.5 w-[3px] rounded-r-full bg-accent" />}
                {n.icon}
                {n.label}
                {n.id === "create" && isGenerating(gen) && <span className="ml-auto h-2 w-2 animate-pulse rounded-full bg-accent-2" />}
                {n.id === "models" && downloading > 0 && (
                  <span className="ml-auto rounded-full bg-accent-2 px-1.5 text-[11px] font-semibold text-bg">{downloading}</span>
                )}
              </button>
            );
          })}
        </div>

        <div className="mt-auto space-y-2 border-t border-line px-3 py-4">
          <div className="flex items-center gap-2 px-2 text-xs text-muted">
            <span className={`h-2 w-2 rounded-full ${rt?.running ? "bg-accent shadow-[0_0_8px_#22d3ee]" : "bg-slow"}`} />
            {rt?.running ? `Engine on · Ollama ${rt.version}` : rt?.installed ? "Engine stopped" : "Engine not installed"}
          </div>
          <Toggle on={sketches} onChange={setSketches} label="Background sketches" />
          <p className="px-2 text-[11px] text-muted/60">v0.1.0</p>
        </div>
      </nav>

      <main className="relative z-10 min-w-0 flex-1 overflow-y-auto">
        <div className={page === "chat" ? "h-full" : "mx-auto max-w-5xl p-8"}>
          {page !== "system" && page !== "chat" && page !== "create" && <RuntimeBanner status={rt} onChange={runtime.reload} />}
          {page === "discover" && <DiscoverPage runtime={rt} onOpenCreate={() => setPage("create")} />}
          {page === "create" && <CreatePage />}
          {page === "models" && <ModelsPage runtime={rt} onChat={(m) => { setChatModel(m); setPage("chat"); }} />}
          {page === "chat" && <ChatPage runtime={rt} startModel={chatModel} onStartModelUsed={() => setChatModel(null)} />}
          {page === "system" && <SystemPage />}
        </div>
      </main>
    </div>
  );
}

export default function App() {
  return (
    <InstallProvider>
      <ImageJobsProvider>
        <Shell />
      </ImageJobsProvider>
    </InstallProvider>
  );
}
