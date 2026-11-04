// Text helpers (pure).
import type { ChangeKind } from "../types";

export function initials(name: string | null | undefined, fallback = "?"): string {
  const parts = (name ?? "").trim().split(/\s+/).filter(Boolean);
  if (parts.length === 0) return fallback;
  const first = parts[0][0] ?? "";
  const last = parts.length > 1 ? parts[parts.length - 1][0] ?? "" : "";
  return (first + last).toUpperCase() || fallback;
}

/** Cuts at a word boundary and appends "…" so the result is at most `max` characters. */
export function truncate(text: string, max: number): string {
  if (text.length <= max) return text;
  const cut = text.slice(0, max - 1);
  const space = cut.lastIndexOf(" ");
  return (space > max / 2 ? cut.slice(0, space) : cut).trimEnd() + "…";
}

/** A stable hue from a string, for initials avatars. */
export function hashHue(s: string): number {
  let h = 0;
  for (let i = 0; i < s.length; i++) h = (h * 31 + s.charCodeAt(i)) >>> 0;
  return h % 360;
}

/** Short name of a change for the row's "changed" tag (FR-022d). */
export function changeLabel(c: ChangeKind): string {
  switch (c.type) {
    case "pipelineFailed":
      return "pipeline failed";
    case "approved":
      return "approved";
    case "changesRequested":
      return "changes requested";
    case "merged":
      return "merged";
    case "statusChanged":
      return c.to ? `status → ${c.to}` : "status";
    case "labelsChanged":
      return "labels";
  }
}
