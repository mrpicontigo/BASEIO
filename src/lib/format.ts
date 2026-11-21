import type { Tier } from "./types";

export function formatBytes(n: number | null | undefined, digits = 1): string {
  if (n == null || !Number.isFinite(n)) return "—";
  if (n < 1024) return `${n} B`;
  const units = ["KB", "MB", "GB", "TB"];
  let v = n;
  let i = -1;
  do { v /= 1024; i++; } while (v >= 1024 && i < units.length - 1);
  return `${v.toFixed(v >= 100 ? 0 : digits)} ${units[i]}`;
}

export function formatParams(b: number): string {
  return b < 1 ? `${Math.round(b * 1000)}M` : `${Number(b.toFixed(1))}B`;
}

export const tierLabel: Record<Tier, string> = {
  great: "Runs great",
  good: "Runs well",
  slow: "Runs slowly",
  too_large: "Too large",
};

export const tierClass: Record<Tier, string> = {
  great: "bg-great/15 text-great border-great/40",
  good: "bg-good/15 text-good border-good/40",
  slow: "bg-slow/15 text-slow border-slow/40",
  too_large: "bg-bad/15 text-bad border-bad/40",
};

export const useCaseLabel: Record<string, string> = {
  chat: "Chat", writing: "Writing", summarization: "Summaries", reasoning: "Reasoning",
  coding: "Coding", multilingual: "Languages", vision: "Images", embeddings: "Search/Embeddings",
};
