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
  great: "bg-great/10 text-great border-great/30",
  good: "bg-good/10 text-good border-good/30",
  slow: "bg-slow/10 text-slow border-slow/30",
  too_large: "bg-bad/10 text-bad border-bad/30",
};

export const useCaseLabel: Record<string, string> = {
  chat: "Chat", writing: "Writing", summarization: "Summaries", reasoning: "Reasoning",
  coding: "Coding", multilingual: "Languages", vision: "Images", embeddings: "Search/Embeddings",
};
