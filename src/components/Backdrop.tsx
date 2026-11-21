import { useEffect, useState } from "react";
import { SKETCHES } from "./sketches";

const KEY = "baseio.backdrop";
const INTERVAL_MS = 14000;

/** Per-user preference, remembered in localStorage (fails safe to "on"). */
export function useBackdropPref(): [boolean, (v: boolean) => void] {
  const [on, setOn] = useState(() => {
    try { return localStorage.getItem(KEY) !== "off"; } catch { return true; }
  });
  const set = (v: boolean) => {
    setOn(v);
    try { localStorage.setItem(KEY, v ? "on" : "off"); } catch { /* ignore */ }
  };
  return [on, set];
}

/**
 * Faint line sketches that slowly cross-fade behind the content.
 * Purely decorative: no pointer events, hidden from screen readers, and it
 * pauses when the window is hidden.
 */
export function Backdrop({ enabled }: { enabled: boolean }) {
  const [i, setI] = useState(() => Math.floor(Math.random() * SKETCHES.length));

  useEffect(() => {
    if (!enabled) return;
    const t = setInterval(() => {
      if (!document.hidden) setI((n) => (n + 1) % SKETCHES.length);
    }, INTERVAL_MS);
    return () => clearInterval(t);
  }, [enabled]);

  if (!enabled) return null;

  return (
    <div aria-hidden className="pointer-events-none fixed inset-0 z-0 overflow-hidden">
      <svg width="0" height="0" className="absolute">
        <filter id="sketchy">
          <feTurbulence type="fractalNoise" baseFrequency="0.035" numOctaves="2" seed="3" />
          <feDisplacementMap in="SourceGraphic" scale="2.2" />
        </filter>
      </svg>
      {SKETCHES.map((s, n) => (
        <div key={s.name} className={`sketch ${n === i ? "is-active" : ""}`}>
          <svg
            viewBox="0 0 200 200"
            className={`absolute -bottom-10 right-[4%] h-[min(78vh,620px)] w-[min(78vh,620px)] ${
              s.color === "cyan" ? "text-accent opacity-[0.10]" : "text-accent-2 opacity-[0.08]"
            }`}
            fill="none"
            stroke="currentColor"
            strokeWidth="1.1"
            strokeLinecap="round"
            strokeLinejoin="round"
            filter="url(#sketchy)"
          >
            {s.body}
          </svg>
        </div>
      ))}
    </div>
  );
}
