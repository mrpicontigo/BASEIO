import { useEffect, useState } from "react";
import SystemPage from "./pages/SystemPage";
import DiscoverPage from "./pages/DiscoverPage";
import ModelsPage from "./pages/ModelsPage";
import ChatPage from "./pages/ChatPage";
import { RuntimeBanner } from "./components/RuntimeBanner";
import { InstallProvider, isActive, useInstalls } from "./lib/installs";
import { api } from "./lib/api";
import { useAsync } from "./lib/useAsync";

type Page = "discover" | "models" | "chat" | "system";

const NAV: { id: Page; label: string; icon: string }[] = [
  { id: "discover", label: "Discover", icon: "◎" },
  { id: "models", label: "My models", icon: "▤" },
  { id: "chat", label: "Chat", icon: "✉" },
  { id: "system", label: "My PC", icon: "▣" },
];

function Shell() {
  const [page, setPage] = useState<Page>("discover");
  const [chatModel, setChatModel] = useState<string | null>(null);
  const runtime = useAsync(api.runtimeStatus);
  const { installs } = useInstalls();
  const downloading = Object.values(installs).filter(isActive).length;

  // Notice if Ollama is started/stopped outside the app.
  useEffect(() => {
    const t = setInterval(() => void runtime.reload(), 15000);
    return () => clearInterval(t);
  }, [runtime.reload]);

  const rt = runtime.data;

  return (
    <div className="flex h-full">
      <nav className="flex w-52 shrink-0 flex-col border-r border-line bg-panel">
        <div className="px-4 py-5">
          <p className="text-lg font-bold tracking-wide">BASEIO</p>
          <p className="text-xs text-muted">Local AI Hub</p>
        </div>
        {NAV.map((n) => (
          <button
            key={n.id}
            onClick={() => setPage(n.id)}
            className={`mx-2 mb-1 flex items-center gap-3 rounded px-3 py-2 text-left text-sm ${
              page === n.id ? "bg-accent/20 text-fg" : "text-muted hover:bg-panel-2 hover:text-fg"
            }`}
          >
            <span className="w-4 text-center">{n.icon}</span>
            {n.label}
            {n.id === "models" && downloading > 0 && (
              <span className="ml-auto rounded-full bg-accent px-1.5 text-xs text-white">{downloading}</span>
            )}
          </button>
        ))}
        <div className="mt-auto px-4 py-3 text-xs text-muted">
          <p>
            <span className={rt?.running ? "text-great" : "text-slow"}>●</span>{" "}
            {rt?.running ? `Engine running (Ollama ${rt.version})` : rt?.installed ? "Engine stopped" : "Engine not installed"}
          </p>
          <p className="mt-1">v0.1.0</p>
        </div>
      </nav>
      <main className="min-w-0 flex-1 overflow-y-auto p-6">
        {page !== "system" && page !== "chat" && <RuntimeBanner status={rt} onChange={runtime.reload} />}
        {page === "discover" && <DiscoverPage runtime={rt} />}
        {page === "models" && <ModelsPage runtime={rt} onChat={(m) => { setChatModel(m); setPage("chat"); }} />}
        {page === "chat" && <ChatPage runtime={rt} startModel={chatModel} onStartModelUsed={() => setChatModel(null)} />}
        {page === "system" && <SystemPage />}
      </main>
    </div>
  );
}

export default function App() {
  return (
    <InstallProvider>
      <Shell />
    </InstallProvider>
  );
}
