import { describe, expect, it } from "vitest";
import { groupByMonthWeek, monthsBack, periodKeys } from "../groups";
import type { ListItem } from "../../types";

// Local-time ISO strings, so the tests don't depend on the machine's time zone.
const at = (y: number, m: number, d: number, h = 12) => new Date(y, m - 1, d, h).toISOString();

function item(n: number, closed: string | null, updated = at(2026, 1, 1)): ListItem {
  return {
    gid: `gid://gitlab/WorkItem/${n}`, kind: "workItems", iid: String(n), reference: `#${n}`,
    title: `T${n}`, webUrl: "", projectPath: "g/p", author: null, createdAt: updated,
    updatedAt: updated, closedAt: closed, labels: [], assignees: [], milestone: null,
    commentCount: 0, upvotes: 0, downvotes: 0, descriptionPreview: "",
  };
}

const shape = (items: ListItem[]) =>
  groupByMonthWeek(items).map((m) => ({
    month: `${m.label} · ${m.count}`,
    weeks: m.weeks.map((w) => `${w.label} · ${w.items.map((i) => i.iid).join(",")}`),
  }));

describe("groupByMonthWeek", () => {
  it("groups by month then Monday–Sunday week, newest first", () => {
    expect(
      shape([
        item(1, at(2026, 11, 4)), // Wed, week Nov 2–8
        item(2, at(2026, 11, 9)), // Mon, week Nov 9–15
        item(3, at(2026, 11, 8, 23)), // Sun 23:00, still week Nov 2–8
        item(4, at(2026, 10, 15)),
      ]),
    ).toEqual([
      { month: "November 2026 · 3", weeks: ["Nov 9 – Nov 15 · 2", "Nov 2 – Nov 8 · 1,3"] },
      { month: "October 2026 · 1", weeks: ["Oct 12 – Oct 18 · 4"] },
    ]);
  });

  it("splits a week that spans two months at the month end", () => {
    // Mon Oct 26 – Sun Nov 1, 2026
    expect(shape([item(1, at(2026, 11, 1)), item(2, at(2026, 10, 26)), item(3, at(2026, 10, 31))])).toEqual([
      { month: "November 2026 · 1", weeks: ["Nov 1 – Nov 1 · 1"] },
      { month: "October 2026 · 2", weeks: ["Oct 26 – Oct 31 · 2,3"] },
    ]);
  });

  it("falls back to the last update when not closed, and keeps input order", () => {
    const g = groupByMonthWeek([item(1, null, at(2026, 3, 10)), item(2, at(2026, 3, 11))]);
    expect(g).toHaveLength(1);
    expect(g[0].weeks[0].items.map((i) => i.iid)).toEqual(["1", "2"]);
    expect(g[0].weeks[0].label).toBe("Mar 9 – Mar 15");
  });

  it("orders years correctly and skips rows without a date", () => {
    const g = groupByMonthWeek([item(1, at(2025, 12, 31)), item(2, at(2026, 1, 2)), item(3, null, "bad")]);
    expect(g.map((m) => m.label)).toEqual(["January 2026", "December 2025"]);
    expect(g[0].weeks[0].label).toBe("Jan 1 – Jan 4");
  });
});

describe("periodKeys", () => {
  it("gives the month and month-clipped Monday week of a local date", () => {
    expect(periodKeys(new Date(2026, 10, 5, 9))).toEqual({ month: "2026-11", week: "2026-11-02", startDay: 2, endDay: 8 });
    expect(periodKeys(new Date(2026, 10, 1, 23, 59))).toEqual({ month: "2026-11", week: "2026-11-01", startDay: 1, endDay: 1 });
    expect(periodKeys(new Date(2026, 9, 31))).toMatchObject({ week: "2026-10-26", endDay: 31 });
  });
  it("matches the keys groupByMonthWeek uses", () => {
    const d = new Date(2026, 10, 4, 12);
    const [m] = groupByMonthWeek([item(1, d.toISOString())]);
    expect([m.key, m.weeks[0].key]).toEqual([periodKeys(d).month, periodKeys(d).week]);
  });
});

describe("monthsBack", () => {
  it("returns the oldest month of an n-month window, across years", () => {
    expect(monthsBack("2026-11", 12)).toBe("2025-12");
    expect(monthsBack("2026-11", 24)).toBe("2024-12");
    expect(monthsBack("2026-01", 1)).toBe("2026-01");
    expect(monthsBack("2026-01", 2)).toBe("2025-12");
  });
});
