// Month → week grouping for "Group by month and week" tabs (FR-038). Pure; tested in
// __tests__/groups.test.ts. All dates are local time.
import type { ListItem } from "../types";

export interface WeekGroup {
  /** `YYYY-MM-DD` of the first day of this (month-clipped) week. */
  key: string;
  /** "Oct 27 – Oct 31". */
  label: string;
  items: ListItem[];
}

export interface MonthGroup {
  /** `YYYY-MM`. */
  key: string;
  /** "November 2026". */
  label: string;
  count: number;
  weeks: WeekGroup[];
}

const MONTH_NAMES = [
  "January", "February", "March", "April", "May", "June",
  "July", "August", "September", "October", "November", "December",
];

const pad = (n: number) => String(n).padStart(2, "0");
const short = (d: Date) => `${MONTH_NAMES[d.getMonth()].slice(0, 3)} ${d.getDate()}`;

/** The date a row is grouped by: closed date, else last update. `null` when neither parses. */
export function groupDate(item: ListItem): Date | null {
  for (const iso of [item.closedAt, item.updatedAt]) {
    if (!iso) continue;
    const t = Date.parse(iso);
    if (!Number.isNaN(t)) return new Date(t);
  }
  return null;
}

export interface PeriodKeys {
  /** `YYYY-MM`. */
  month: string;
  /** `YYYY-MM-DD` of the first day of the month-clipped Monday–Sunday week. */
  week: string;
  startDay: number;
  endDay: number;
}

/** The month and (month-clipped) week a local date falls in. */
export function periodKeys(d: Date): PeriodKeys {
  const y = d.getFullYear();
  const m = d.getMonth();
  const month = `${y}-${pad(m + 1)}`;
  const lastDay = new Date(y, m + 1, 0).getDate();
  const mondayDay = d.getDate() - ((d.getDay() + 6) % 7);
  const startDay = Math.max(1, mondayDay);
  const endDay = Math.min(lastDay, mondayDay + 6);
  return { month, week: `${month}-${pad(startDay)}`, startDay, endDay };
}

/** The oldest month key of an `n`-month window ending at `monthKey`: (`2026-11`, 12) → `2025-12`. */
export function monthsBack(monthKey: string, n: number): string {
  const [y, m] = monthKey.split("-").map(Number);
  const d = new Date(y, m - 1 - (n - 1), 1);
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}`;
}

/**
 * Groups rows by month, then by Monday–Sunday week clipped to the month, newest first.
 * Rows keep their input order inside a week. Rows without a usable date are left out.
 */
export function groupByMonthWeek(items: readonly ListItem[]): MonthGroup[] {
  const months = new Map<string, MonthGroup>();
  const weeks = new Map<string, WeekGroup>();
  for (const item of items) {
    const d = groupDate(item);
    if (!d) continue;
    const y = d.getFullYear();
    const m = d.getMonth();
    const { month: monthKey, week: weekKey, startDay, endDay } = periodKeys(d);
    let month = months.get(monthKey);
    if (!month) {
      month = { key: monthKey, label: `${MONTH_NAMES[m]} ${y}`, count: 0, weeks: [] };
      months.set(monthKey, month);
    }
    let week = weeks.get(weekKey);
    if (!week) {
      week = {
        key: weekKey,
        label: `${short(new Date(y, m, startDay))} – ${short(new Date(y, m, endDay))}`,
        items: [],
      };
      weeks.set(weekKey, week);
      month.weeks.push(week);
    }
    week.items.push(item);
    month.count++;
  }
  const out = [...months.values()].sort((a, b) => b.key.localeCompare(a.key));
  for (const month of out) month.weeks.sort((a, b) => b.key.localeCompare(a.key));
  return out;
}
