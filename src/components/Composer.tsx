import { useEffect, useRef, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { api } from "../lib/api";
import { formatBytes } from "../lib/format";
import type { AppError, Attachment, ModelInfo } from "../lib/types";

export interface PendingFile { id: string; name: string; att?: Attachment; error?: string }

const icon = (d: React.ReactNode, cls = "h-5 w-5") => (
  <svg viewBox="0 0 24 24" className={cls} fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">{d}</svg>
);

export function FileChip({ name, att, error, onRemove }: { name: string; att?: Attachment; error?: string; onRemove?: () => void }) {
  const isImg = att?.kind === "image" && att.data_base64;
  return (
    <div className={`group relative flex items-center gap-2 rounded-xl border px-2 py-1.5 text-xs ${error ? "border-bad/40 bg-bad/5" : "border-line bg-bg/60"}`}>
      {isImg ? (
        <img src={`data:${att!.mime};base64,${att!.data_base64}`} alt="" className="h-9 w-9 rounded-lg object-cover" />
      ) : (
        <span className={`flex h-9 w-9 items-center justify-center rounded-lg ${error ? "bg-bad/10 text-bad" : "bg-accent-2/10 text-accent-2"}`}>
          {!att && !error ? <span className="h-4 w-4 animate-spin rounded-full border-2 border-line border-t-accent" /> : icon(<><path d="M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8z" /><path d="M14 3v5h5" /></>, "h-4 w-4")}
        </span>
      )}
      <div className="min-w-0 max-w-44">
        <p className="truncate font-medium text-fg">{name}</p>
        <p className={`truncate ${error ? "text-bad" : "text-muted"}`}>
          {error ?? (att ? `${formatBytes(att.size_bytes)}${att.truncated ? " · first part only" : ""}` : "Reading…")}
        </p>
      </div>
      {onRemove && (
        <button onClick={onRemove} title="Remove" className="absolute -right-1.5 -top-1.5 hidden h-5 w-5 items-center justify-center rounded-full border border-line bg-panel-2 text-muted hover:text-fg group-hover:flex">×</button>
      )}
    </div>
  );
}

interface Props {
  disabled: boolean;
  streaming: boolean;
  model: string;
  info: ModelInfo | null;
  onSend: (text: string, atts: Attachment[]) => Promise<boolean>;
  onStop: () => void;
  autoFocus?: boolean;
}

/** ChatGPT-style message box: autosizing input, attachments, drag & drop. */
export function Composer({ disabled, streaming, model, info, onSend, onStop, autoFocus }: Props) {
  const [text, setText] = useState("");
  const [files, setFiles] = useState<PendingFile[]>([]);
  const [dragging, setDragging] = useState(false);
  const ta = useRef<HTMLTextAreaElement>(null);
  const vision = info?.capabilities.includes("vision") ?? false;

  useEffect(() => {
    const el = ta.current;
    if (!el) return;
    el.style.height = "auto";
    el.style.height = `${Math.min(el.scrollHeight, 220)}px`;
  }, [text]);

  useEffect(() => { if (autoFocus) ta.current?.focus(); }, [autoFocus]);

  const addPaths = (paths: string[]) => {
    for (const path of paths.slice(0, 10)) {
      const id = `${path}-${Math.random()}`;
      const name = path.split(/[\\/]/).pop() ?? path;
      setFiles((f) => [...f, { id, name }]);
      api.readAttachment(path)
        .then((att) => setFiles((f) => f.map((x) => (x.id === id ? { ...x, att } : x))))
        .catch((e: AppError) => setFiles((f) => f.map((x) => (x.id === id ? { ...x, error: e.message } : x))));
    }
  };

  // Drag & drop files from Explorer onto the window.
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    getCurrentWebview()
      .onDragDropEvent((e) => {
        const p = e.payload;
        if (p.type === "over" || p.type === "enter") setDragging(true);
        else if (p.type === "leave") setDragging(false);
        else if (p.type === "drop") { setDragging(false); addPaths(p.paths); }
      })
      .then((u) => { unlisten = u; })
      .catch(() => { /* not running inside Tauri */ });
    return () => unlisten?.();
  }, []);

  const pick = async () => {
    try {
      const exts = await api.attachmentExtensions();
      const res = await open({ multiple: true, filters: [{ name: "Images, PDFs, documents, code", extensions: exts }] });
      if (res) addPaths(Array.isArray(res) ? res : [res]);
    } catch { /* dialog closed */ }
  };

  const ready = files.filter((f) => f.att).map((f) => f.att!);
  const reading = files.some((f) => !f.att && !f.error);
  const hasImage = ready.some((a) => a.kind === "image");
  const blocked = hasImage && !vision;
  const canSend = !disabled && !streaming && !reading && !blocked && (text.trim().length > 0 || ready.length > 0);

  const send = async () => {
    if (!canSend) return;
    const ok = await onSend(text, ready);
    if (ok) { setText(""); setFiles([]); }
  };

  return (
    <div className="relative">
      {dragging && (
        <div className="pointer-events-none absolute -inset-2 z-20 flex items-center justify-center rounded-[28px] border-2 border-dashed border-accent bg-bg/80 text-sm text-accent">
          Drop files to attach
        </div>
      )}
      <div className="rounded-[26px] border border-line bg-panel-2/90 p-2.5 shadow-[0_8px_40px_-12px_rgba(0,0,0,.6)] backdrop-blur-xl transition focus-within:border-accent/40">
        {files.length > 0 && (
          <div className="flex flex-wrap gap-2 px-1.5 pb-2 pt-1">
            {files.map((f) => (
              <FileChip key={f.id} name={f.name} att={f.att} error={f.error} onRemove={() => setFiles((x) => x.filter((y) => y.id !== f.id))} />
            ))}
          </div>
        )}
        <textarea
          ref={ta}
          rows={1}
          value={text}
          disabled={disabled}
          onChange={(e) => setText(e.target.value)}
          onKeyDown={(e) => { if (e.key === "Enter" && !e.shiftKey) { e.preventDefault(); void send(); } }}
          onPaste={() => { /* files are attached via button or drag & drop */ }}
          placeholder={disabled ? "Start the AI engine to chat" : `Message ${model || "a model"}`}
          className="block max-h-[220px] w-full resize-none bg-transparent px-3 py-2 text-[15px] outline-none placeholder:text-muted/70"
        />
        <div className="flex items-center justify-between px-1 pt-1">
          <button
            onClick={pick}
            disabled={disabled}
            title="Attach images, PDFs, Word documents or text/code files"
            className="flex h-9 w-9 items-center justify-center rounded-full text-muted transition hover:bg-line hover:text-fg disabled:opacity-40"
          >
            {icon(<path d="m21.4 11.6-8.5 8.5a5.5 5.5 0 0 1-7.8-7.8l8.5-8.5a3.7 3.7 0 0 1 5.2 5.2l-8.5 8.5a1.8 1.8 0 0 1-2.6-2.6l7.8-7.8" />)}
          </button>
          {streaming ? (
            <button onClick={onStop} title="Stop" className="flex h-9 w-9 items-center justify-center rounded-full bg-fg text-bg transition hover:brightness-90">
              <span className="h-3 w-3 rounded-[3px] bg-bg" />
            </button>
          ) : (
            <button
              onClick={send}
              disabled={!canSend}
              title="Send"
              className="flex h-9 w-9 items-center justify-center rounded-full bg-accent text-bg transition hover:brightness-110 disabled:bg-line disabled:text-muted"
            >
              {icon(<path d="M12 19V5M5 12l7-7 7 7" />)}
            </button>
          )}
        </div>
      </div>
      <p className={`mt-2 text-center text-[11px] ${blocked ? "text-bad" : "text-muted/80"}`}>
        {blocked
          ? `${model} can't see images. Pick a model marked "Sees images" or remove the image.`
          : info && !vision
            ? `${model} reads and writes text and documents. It can't see or create images.`
            : info && vision
              ? `${model} can read text, documents and images. Runs fully on this computer.`
              : "Runs fully on this computer."}
      </p>
    </div>
  );
}
