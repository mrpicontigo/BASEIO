import { useEffect, useRef, useState } from "react";
import { api } from "../lib/api";
import { useAsync } from "../lib/useAsync";
import { Button, ErrorBox } from "../components/ui";
import type { AppError, Conversation, Message, RuntimeStatus } from "../lib/types";

export default function ChatPage({
  runtime, startModel, onStartModelUsed,
}: { runtime: RuntimeStatus | null; startModel: string | null; onStartModelUsed: () => void }) {
  const running = runtime?.running ?? false;
  const convs = useAsync(api.listConversations);
  const installed = useAsync(() => (running ? api.listInstalled() : Promise.resolve([])), [running]);
  const [active, setActive] = useState<Conversation | null>(null);
  const [messages, setMessages] = useState<Message[]>([]);
  const [draft, setDraft] = useState("");
  const [streaming, setStreaming] = useState<string | null>(null);
  const [error, setError] = useState<AppError | null>(null);
  const [newModel, setNewModel] = useState("");
  const bottom = useRef<HTMLDivElement>(null);

  const open = async (c: Conversation) => {
    setActive(c);
    setError(null);
    try { setMessages(await api.getMessages(c.id)); } catch (e) { setError(e as AppError); }
  };

  const newChat = async (model: string) => {
    try {
      const c = await api.createConversation(model);
      await convs.reload();
      setActive(c);
      setMessages([]);
    } catch (e) { setError(e as AppError); }
  };

  // Arrived via "Chat" on the Models page.
  useEffect(() => {
    if (startModel) { void newChat(startModel); onStartModelUsed(); }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [startModel]);

  useEffect(() => {
    if (!newModel && installed.data?.length) setNewModel(installed.data[0].name);
  }, [installed.data, newModel]);

  useEffect(() => { bottom.current?.scrollIntoView({ behavior: "smooth" }); }, [messages, streaming]);

  const send = async () => {
    if (!active || !draft.trim() || streaming !== null) return;
    const text = draft.trim();
    setDraft("");
    setError(null);
    setMessages((m) => [...m, { id: -Date.now(), conversation_id: active.id, role: "user", content: text, created_at: Date.now() }]);
    setStreaming("");
    try {
      await api.sendMessage(active.id, text, (e) => {
        if (e.event === "token") setStreaming((s) => (s ?? "") + e.content);
      });
    } catch (e) {
      const err = e as AppError;
      if (err.kind !== "cancelled") setError(err);
    } finally {
      setStreaming(null);
      setMessages(await api.getMessages(active.id).catch(() => messages));
      void convs.reload();
    }
  };

  const remove = async (c: Conversation) => {
    if (!window.confirm(`Delete chat "${c.title}"?`)) return;
    await api.deleteConversation(c.id).catch((e) => setError(e as AppError));
    if (active?.id === c.id) { setActive(null); setMessages([]); }
    void convs.reload();
  };

  const modelMissing = active && installed.data && !installed.data.some((m) => m.name === active.model);

  return (
    <div className="-m-6 flex h-[calc(100%+3rem)]">
      <aside className="flex w-60 shrink-0 flex-col border-r border-line bg-panel">
        <div className="space-y-2 border-b border-line p-3">
          <select
            className="w-full rounded border border-line bg-panel-2 px-2 py-1.5 text-sm"
            value={newModel}
            onChange={(e) => setNewModel(e.target.value)}
            disabled={!installed.data?.length}
          >
            {!installed.data?.length && <option value="">No models installed</option>}
            {installed.data?.map((m) => <option key={m.name} value={m.name}>{m.name}</option>)}
          </select>
          <Button onClick={() => newChat(newModel)} disabled={!newModel}>+ New chat</Button>
        </div>
        <div className="flex-1 overflow-y-auto p-2">
          {convs.data?.map((c) => (
            <div
              key={c.id}
              className={`group mb-1 flex cursor-pointer items-center rounded px-2 py-1.5 text-sm ${active?.id === c.id ? "bg-accent/20" : "hover:bg-panel-2"}`}
              onClick={() => open(c)}
            >
              <div className="min-w-0 flex-1">
                <p className="truncate">{c.title}</p>
                <p className="truncate text-xs text-muted">{c.model}</p>
              </div>
              <button
                className="ml-1 hidden text-muted hover:text-bad group-hover:block"
                title="Delete chat"
                onClick={(e) => { e.stopPropagation(); void remove(c); }}
              >×</button>
            </div>
          ))}
        </div>
      </aside>

      <section className="flex min-w-0 flex-1 flex-col">
        {!active ? (
          <div className="m-auto max-w-sm text-center text-sm text-muted">
            {running ? "Pick a model and start a new chat. Everything stays on this computer." : "Start the AI engine to chat."}
          </div>
        ) : (
          <>
            <div className="border-b border-line px-6 py-3 text-sm">
              <span className="font-medium">{active.title}</span> <span className="font-mono text-xs text-muted">{active.model}</span>
            </div>
            <div className="flex-1 space-y-4 overflow-y-auto px-6 py-4">
              {messages.map((m) => <Bubble key={m.id} role={m.role} content={m.content} />)}
              {streaming !== null && <Bubble role="assistant" content={streaming || "…"} />}
              {error && <ErrorBox error={error} />}
              {modelMissing && <ErrorBox error={{ kind: "not_found", message: `${active.model} is no longer installed.` }} />}
              <div ref={bottom} />
            </div>
            <div className="border-t border-line p-4">
              <div className="flex gap-2">
                <textarea
                  className="flex-1 resize-none rounded border border-line bg-panel-2 px-3 py-2 text-sm outline-none focus:border-accent"
                  rows={2}
                  placeholder={running ? "Type a message (Enter to send, Shift+Enter for a new line)" : "Start the AI engine first"}
                  value={draft}
                  disabled={!running}
                  onChange={(e) => setDraft(e.target.value)}
                  onKeyDown={(e) => { if (e.key === "Enter" && !e.shiftKey) { e.preventDefault(); void send(); } }}
                />
                {streaming !== null
                  ? <Button variant="ghost" onClick={() => api.cancelChat(active.id)}>Stop</Button>
                  : <Button onClick={send} disabled={!running || !draft.trim()}>Send</Button>}
              </div>
            </div>
          </>
        )}
      </section>
    </div>
  );
}

function Bubble({ role, content }: { role: string; content: string }) {
  const user = role === "user";
  return (
    <div className={`flex ${user ? "justify-end" : "justify-start"}`}>
      <div className={`max-w-[80%] whitespace-pre-wrap rounded-lg px-4 py-2 text-sm ${user ? "bg-accent/25" : "bg-panel-2"}`}>
        {content}
      </div>
    </div>
  );
}
