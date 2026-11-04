import { describe, expect, it } from "vitest";
import { formatDueDate, isOverdue, relativeTime } from "../time";
import { labelTextColor, splitScopedLabel } from "../labels";
import { initials, truncate } from "../text";

const NOW = Date.parse("2026-11-03T12:00:00Z");

describe("relativeTime", () => {
  it("formats ranges", () => {
    expect(relativeTime("2026-11-03T11:59:30Z", NOW)).toBe("just now");
    expect(relativeTime("2026-11-03T11:55:00Z", NOW)).toBe("5m ago");
    expect(relativeTime("2026-11-03T09:00:00Z", NOW)).toBe("3h ago");
    expect(relativeTime("2026-11-01T12:00:00Z", NOW)).toBe("2d ago");
    expect(relativeTime("2026-08-01T12:00:00Z", NOW)).toBe("3mo ago");
    expect(relativeTime("2024-10-01T12:00:00Z", NOW)).toBe("2y ago");
  });
  it("handles future, null and invalid", () => {
    expect(relativeTime("2026-11-04T12:00:00Z", NOW)).toBe("just now");
    expect(relativeTime(null, NOW)).toBe("");
    expect(relativeTime("nope", NOW)).toBe("");
  });
});

describe("splitScopedLabel", () => {
  it("splits on the last ::", () => {
    expect(splitScopedLabel("a::b::c")).toEqual({ scope: "a::b", value: "c" });
    expect(splitScopedLabel("state::Ready to Merge")).toEqual({ scope: "state", value: "Ready to Merge" });
  });
  it("leaves unscoped labels alone", () => {
    expect(splitScopedLabel("Frontend")).toEqual({ scope: null, value: "Frontend" });
    expect(splitScopedLabel("::x")).toEqual({ scope: null, value: "::x" });
    expect(splitScopedLabel("x::")).toEqual({ scope: null, value: "x::" });
  });
});

describe("labelTextColor", () => {
  it("uses GitLab's text colour when present", () => {
    expect(labelTextColor("#ff0000", "#FFFFFF")).toBe("#FFFFFF");
  });
  it("falls back to contrast", () => {
    expect(labelTextColor("#c4f4c4", null)).toBe("#1f1e24");
    expect(labelTextColor("#013220", "")).toBe("#ffffff");
    expect(labelTextColor("#fff")).toBe("#1f1e24");
    expect(labelTextColor("bogus")).toBe("#1f1e24");
  });
});

describe("dates", () => {
  const now = new Date(2026, 10, 3, 12);
  it("isOverdue", () => {
    expect(isOverdue("2026-11-02", now)).toBe(true);
    expect(isOverdue("2026-11-03", now)).toBe(false);
    expect(isOverdue("2026-12-01", now)).toBe(false);
    expect(isOverdue(null, now)).toBe(false);
  });
  it("formatDueDate", () => {
    expect(formatDueDate("2026-11-05", now)).toBe("Nov 5");
    expect(formatDueDate("2027-01-03", now)).toBe("Jan 3, 2027");
    expect(formatDueDate(null, now)).toBe("");
  });
});

describe("text", () => {
  it("initials", () => {
    expect(initials("Dusan Jankov")).toBe("DJ");
    expect(initials("cher")).toBe("C");
    expect(initials("")).toBe("?");
  });
  it("truncate at word boundary", () => {
    expect(truncate("short", 10)).toBe("short");
    const t = truncate("hello wonderful world of tests", 16);
    expect(t).toBe("hello wonderful…");
    expect(t.length).toBeLessThanOrEqual(16);
  });
});
