import { useEffect, useMemo, useRef, useState } from "react";
import { api } from "../lib/api";
import { useAsync } from "../lib/useAsync";
import { Composer, FileChip } from "../components/Composer";
import { Markdown } from "../components/Markdown";
import { ErrorBox, Logo } from "../components/ui";
import type { AppError, Attachment, Conversation, Message, ModelInfo, RuntimeStatus } from "../lib/types";

const SUGGESTIONS = [
  "Explain how a rocket gets into orbit, simply",
  "Write a polite email asking for a deadline extension",
  "Summarise the file I attach in 5 bullet points",
  "Help me fix a bug in my Python code",
];

const CAP_LABEL: Record<string, string> = { vision: "Sees images", thinking: "Thinks first", tools: "Tools" };

function groupConversations(convs: Conversation[]) {
  const day = 86_400_000;
  const startToday = new Date().setHours(0, 0, 0, 0);
  const groups: [string, Conversation[]][] = [["Today", []], ["Previous 7 days", []], ["Older", []]];
  for (const c of convs) {
    const g = c.updated_at >= startToday ? 0 : c.updated_at >= startToday - 7 * day ? 1 : 2;
    groups[g][1].push(c);
  }
  return groups.filter(([, cs]) => cs.length > 0);
}

function ModelPicker({
  value, models, infos, onChange, disabled,
}: { value: string; models: string[]; infos: Record<string, ModelInfo>; onChange: (m: string) => void; disabled: boolean }) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const close = (e: MouseEvent) => { if (!ref.current?.contains(e.target as Node)) setOpen(false); };
    document.addEventListener("mousedown", close);
    return () => document.removeEventListener("mousedown", close);
  }, []);
  return (
    <div ref={ref} className="relative">
      <button
        disabled={disabled}
        onClick={() => setOpen((o) => !o)}
        className="flex items-center gap-2 rounded-xl px-3 py-2 text-[15px] font-semibold transition hover:bg-panel-2 disabled:opacity-50"
      >
        {value || "Choose a model"}
        <svg viewBox="0 0 24 24" className="h-4 w-4 text-muted" fill="none" stroke="currentColor" strokeWidth="2"><path d="m6 9 6 6 6-6" /></svg>
      </button>
      {open && (
        <div className="absolute left-0 top-full z-30 mt-1 w-80 rounded-2xl border border-line bg-panel-2 p-1.5 shadow-2xl">
          {models.length === 0 && <p className="p-3 text-sm text-muted">No models installed. Get one on the Discover page.</p>}
          {models.map((m) => {
            const caps = infos[m]?.capabilities ?? [];
            return (
              <button
                key={m}
                onClick={() => { onChange(m); setOpen(false); }}
                className={`flex w-full items-center justify-between gap-3 rounded-xl px-3 py-2.5 text-left text-sm transition hover:bg-line/60 ${m === value ? "text-accent" : ""}`}
              >
                <span>
                  <span className="block font-medium">{m}</span>
                  <span className="mt-0.5 flex flex-wrap gap-1">
                    <span className="rounded-full bg-bg/60 px-2 py-0.5 text-[10px] text-muted">Text</span>
                    {caps.filter((c) => CAP_LABEL[c]).map((c) => (
                      <span key={c} className={`rounded-full px-2 py-0.5 text-[10px] ${c === "vision" ? "bg-accent-2/10 text-accent-2" : "bg-accent/10 text-accent"}`}>{CAP_LABEL[c]}</span>
                    ))}
                    {infos[m]?.parameter_size && <span className="px-1 text-[10px] text-muted">{infos[m].parameter_size}</span>}
                  </span>
                </span>
                {m === value && <span>✓</span>}
              </button>
            );
          })}
        </div>
      )}
    </div>
  );
}

function CopyButton({ text }: { text: string }) {
  const [done, setDone] = useState(false);
  return (
    <button
      onClick={async () => { try { await navigator.clipboard.writeText(text); setDone(true); setTimeout(() => setDone(false), 1500); } catch { /* ignore */ } }}
      className="rounded-md px-2 py-1 text-xs text-muted transition hover:bg-panel-2 hover:text-fg"
    >{done ? "Copied ✓" : "Copy"}</button>
  );
}

function MessageView({ m }: { m: { role: string; content: string; attachments: Attachment[] } }) {
  if (m.role === "user") {
    return (
      <div className="flex flex-col items-end gap-2">
        {m.attachments.length > 0 && (
          <div className="flex flex-wrap justify-end gap-2">
            {m.attachments.map((a, i) => <FileChip key={i} name={a.name} att={a} />)}
          </div>
        )}
        {m.content && (
          <div className="max-w-[85%] whitespace-pre-wrap rounded-3xl bg-panel-2 px-5 py-3 text-[15px] leading-7">{m.content}</div>
        )}
      </div>
    );
  }
  return (
    <div className="group flex gap-4">
      <div className="mt-1 shrink-0"><Logo size={26} /></div>
      <div className="min-w-0 flex-1">
        <Markdown text={m.content} />
        <div className="mt-1 opacity-0 transition group-hover:opacity-100"><CopyButton text={m.content} /></div>
      </div>
    </div>
  );
}

export default function ChatPage({
  runtime, startModel, onStartModelUsed,
}: { runtime: RuntimeStatus | null; startModel: string | null; onStartModelUsed: () => void }) {
  const running = runtime?.running ?? false;
  const convs = useAsync(api.listConversations);
  const installed = useAsync(() => (running ? api.listInstalled() : Promise.resolve([])), [running]);
  const models = useMemo(() => (installed.data ?? []).map((m) => m.name), [installed.data]);
  const [infos, setInfos] = useState<Record<string, ModelInfo>>({});
  const [activeId, setActiveId] = useState<number | null>(null);
  const [model, setModel] = useState("");
  const [messages, setMessages] = useState<Message[]>([]);
  const [streaming, setStreaming] = useState<string | null>(null);
  const [thinking, setThinking] = useState(false);
  const [error, setError] = useState<AppError | null>(null);
  const scroller = useRef<HTMLDivElement>(null);

  const active = convs.data?.find((c) => c.id === activeId) ?? null;

  // Capabilities for every installed model (for the picker and warnings).
  useEffect(() => {
    for (const m of models) {
      if (!infos[m]) api.modelInfo(m).then((i) => setInfos((s) => ({ ...s, [m]: i }))).catch(() => {});
    }
  }, [models]); // eslint-disable-line react-hooks/exhaustive-deps

  useEffect(() => {
    if (!model && models.length) setModel(models[0]);
  }, [models, model]);

  // Arrived via "Chat" on the Models page: start a fresh chat with that model.
  useEffect(() => {
    if (startModel) { newChat(); setModel(startModel); onStartModelUsed(); }
  }, [startModel]); // eslint-disable-line react-hooks/exhaustive-deps

  useEffect(() => {
    const el = scroller.current;
    if (el) el.scrollTo({ top: el.scrollHeight, behavior: "smooth" });
  }, [messages, streaming]);

  const newChat = () => { setActiveId(null); setMessages([]); setError(null); };

  const open = async (c: Conversation) => {
    setActiveId(c.id);
    setModel(c.model);
    setError(null);
    try { setMessages(await api.getMessages(c.id)); } catch (e) { setError(e as AppError); }
  };

  const changeModel = async (m: string) => {
    setModel(m);
    if (activeId != null) {
      try { await api.setConversationModel(activeId, m); void convs.reload(); } catch (e) { setError(e as AppError); }
    }
  };

  const send = async (text: string, atts: Attachment[]): Promise<boolean> => {
    if (!model) return false;
    setError(null);
    let id = activeId;
    try {
      if (id == null) {
        const c = await api.createConversation(model);
        id = c.id;
        setActiveId(id);
      }
    } catch (e) { setError(e as AppError); return false; }

    const convId = id;
    setMessages((m) => [...m, { id: -Date.now(), conversation_id: convId, role: "user", content: text.trim(), created_at: Date.now(), attachments: atts }]);
    setStreaming("");
    setThinking(false);
    void (async () => {
      try {
        await api.sendMessage(convId, text, atts, (e) => {
          if (e.event === "token") setStreaming((s) => (s ?? "") + e.content);
          else if (e.event === "thinking") setThinking(true);
        });
      } catch (e) {
        const err = e as AppError;
        if (err.kind !== "cancelled") setError(err);
      } finally {
        setStreaming(null);
        setThinking(false);
        try { setMessages(await api.getMessages(convId)); } catch { /* keep optimistic view */ }
        void convs.reload(); // picks up the auto-generated title
      }
    })();
    return true;
  };

  const remove = async (c: Conversation) => {
    if (!window.confirm(`Delete chat "${c.title}"?`)) return;
    try { await api.deleteConversation(c.id); } catch (e) { setError(e as AppError); }
    if (activeId === c.id) newChat();
    void convs.reload();
  };

  const info = infos[model] ?? null;
  const empty = messages.length === 0 && streaming === null;
  const composer = (
    <Composer
      disabled={!running || !model}
      streaming={streaming !== null}
      model={model}
      info={info}
      onSend={send}
      onStop={() => { if (activeId != null) void api.cancelChat(activeId); }}
      autoFocus
    />
  );

  return (
    <div className="flex h-full">
      {/* conversation history */}
      <aside className="flex w-64 shrink-0 flex-col border-r border-line bg-panel/60 backdrop-blur-md">
        <div className="p-3">
          <button
            onClick={newChat}
            className="flex w-full items-center gap-2 rounded-xl border border-line px-3 py-2.5 text-sm font-medium transition hover:border-accent/40 hover:text-accent"
          >
            <svg viewBox="0 0 24 24" className="h-4 w-4" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round"><path d="M12 5v14M5 12h14" /></svg>
            New chat
          </button>
        </div>
        <div className="flex-1 overflow-y-auto px-2 pb-3">
          {groupConversations(convs.data ?? []).map(([label, cs]) => (
            <div key={label} className="mb-3">
              <p className="px-3 pb-1 pt-2 text-[11px] font-medium text-muted/80">{label}</p>
              {cs.map((c) => (
                <div
                  key={c.id}
                  onClick={() => open(c)}
                  className={`group flex cursor-pointer items-center rounded-lg px-3 py-2 text-sm transition ${activeId === c.id ? "bg-panel-2 text-fg" : "text-fg/80 hover:bg-panel-2/60"}`}
                >
                  <span className="min-w-0 flex-1 truncate">{c.title}</span>
                  <button
                    className="ml-1 hidden rounded px-1 text-muted hover:text-bad group-hover:block"
                    title="Delete chat"
                    onClick={(e) => { e.stopPropagation(); void remove(c); }}
                  >×</button>
                </div>
              ))}
            </div>
          ))}
          {convs.data?.length === 0 && <p className="px-3 py-2 text-xs text-muted">Your chats will appear here.</p>}
        </div>
      </aside>

      {/* conversation */}
      <section className="relative flex min-w-0 flex-1 flex-col">
        <header className="flex items-center justify-between gap-3 px-4 py-2.5">
          <ModelPicker value={model} models={models} infos={infos} onChange={changeModel} disabled={!running} />
          {active && <span className="truncate text-sm text-muted">{active.title}</span>}
        </header>

        {empty ? (
          <div className="flex flex-1 flex-col items-center justify-center px-6 pb-16">
            <Logo size={44} />
            <h1 className="mb-8 mt-4 text-3xl font-semibold tracking-tight">
              {running ? (models.length ? "What can I help with?" : "Install a model to start chatting") : "Start the AI engine to chat"}
            </h1>
            <div className="w-full max-w-3xl">{composer}</div>
            {running && models.length > 0 && (
              <div className="mt-6 flex max-w-3xl flex-wrap justify-center gap-2">
                {SUGGESTIONS.map((s) => (
                  <button key={s} onClick={() => void send(s, [])} className="rounded-full border border-line px-4 py-2 text-sm text-muted transition hover:border-accent/40 hover:text-fg">
                    {s}
                  </button>
                ))}
              </div>
            )}
            {error && <div className="mt-6 w-full max-w-3xl"><ErrorBox error={error} /></div>}
          </div>
        ) : (
          <>
            <div ref={scroller} className="flex-1 overflow-y-auto">
              <div className="mx-auto max-w-3xl space-y-8 px-6 py-6">
                {messages.map((m) => <MessageView key={m.id} m={m} />)}
                {streaming !== null && (
                  <div className="flex gap-4">
                    <div className="mt-1 shrink-0"><Logo size={26} /></div>
                    <div className="min-w-0 flex-1">
                      {streaming ? <Markdown text={streaming} /> : (
                        <span className="inline-flex items-center gap-2 text-sm text-muted">
                          <span className="h-2 w-2 animate-pulse rounded-full bg-accent" />
                          {thinking ? "Thinking…" : "Working…"}
                        </span>
                      )}
                    </div>
                  </div>
                )}
                {error && <ErrorBox error={error} />}
              </div>
            </div>
            <div className="mx-auto w-full max-w-3xl px-6 pb-4">{composer}</div>
          </>
        )}
      </section>
    </div>
  );
}
