import { memo, useState, type ReactNode } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { openUrl } from "@tauri-apps/plugin-opener";

/** Collapse whitespace-only lines and runs of blank lines models sometimes emit. */
export function tidy(s: string): string {
  return s.replace(/[ \t]+$/gm, "").replace(/\n{3,}/g, "\n\n").trim();
}

function CodeBlock({ children, lang }: { children: ReactNode; lang?: string }) {
  const [copied, setCopied] = useState(false);
  const text = String(children).replace(/\n$/, "");
  const copy = async () => {
    try { await navigator.clipboard.writeText(text); setCopied(true); setTimeout(() => setCopied(false), 1500); } catch { /* ignore */ }
  };
  return (
    <div className="my-3 overflow-hidden rounded-xl border border-line bg-bg/80">
      <div className="flex items-center justify-between border-b border-line px-3 py-1.5 text-[11px] text-muted">
        <span>{lang || "code"}</span>
        <button onClick={copy} className="hover:text-accent">{copied ? "Copied ✓" : "Copy"}</button>
      </div>
      <pre className="overflow-x-auto p-3 text-[13px] leading-relaxed"><code>{text}</code></pre>
    </div>
  );
}

/** Safe markdown (no raw HTML). Links open in the system browser. */
export const Markdown = memo(function Markdown({ text }: { text: string }) {
  return (
    <div className="md text-[15px] leading-7">
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        components={{
          code({ className, children }) {
            const lang = /language-(\w+)/.exec(className || "")?.[1];
            const block = !!lang || String(children).includes("\n");
            return block
              ? <CodeBlock lang={lang}>{children}</CodeBlock>
              : <code className="rounded bg-panel-2 px-1.5 py-0.5 text-[13px] text-accent">{children}</code>;
          },
          pre: ({ children }) => <>{children}</>,
          a: ({ href, children }) => (
            <a
              href={href}
              className="text-accent underline decoration-accent/40 underline-offset-2 hover:decoration-accent"
              onClick={(e) => { e.preventDefault(); if (href && /^https?:\/\//.test(href)) void openUrl(href); }}
            >{children}</a>
          ),
          img: ({ alt }) => <span className="text-muted">[image: {alt || "not available"}]</span>,
          table: ({ children }) => <div className="my-3 overflow-x-auto"><table className="w-full border-collapse text-sm">{children}</table></div>,
          th: ({ children }) => <th className="border-b border-line px-3 py-2 text-left font-semibold">{children}</th>,
          td: ({ children }) => <td className="border-b border-line/60 px-3 py-2">{children}</td>,
        }}
      >
        {tidy(text)}
      </ReactMarkdown>
    </div>
  );
});
