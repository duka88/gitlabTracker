// Time formatting helpers (pure; tested in __tests__/util.test.ts).

const MIN = 60_000;
const HOUR = 60 * MIN;
const DAY = 24 * HOUR;

/** "just now", "5m ago", "3h ago", "2d ago", "3mo ago", "1y ago". Future dates read "just now". */
export function relativeTime(iso: string | null | undefined, now: number = Date.now()): string {
  if (!iso) return "";
  const t = Date.parse(iso);
  if (Number.isNaN(t)) return "";
  const diff = now - t;
  if (diff < MIN) return "just now";
  if (diff < HOUR) return `${Math.floor(diff / MIN)}m ago`;
  if (diff < DAY) return `${Math.floor(diff / HOUR)}h ago`;
  const days = Math.floor(diff / DAY);
  if (days < 30) return `${days}d ago`;
  if (days < 365) return `${Math.floor(days / 30)}mo ago`;
  return `${Math.floor(days / 365)}y ago`;
}

function localDateKey(d: Date): string {
  const m = String(d.getMonth() + 1).padStart(2, "0");
  const day = String(d.getDate()).padStart(2, "0");
  return `${d.getFullYear()}-${m}-${day}`;
}

/** True when the `YYYY-MM-DD` due date is before today (local time). Due today is not overdue. */
export function isOverdue(dueDate: string | null | undefined, now: Date = new Date()): boolean {
  if (!dueDate) return false;
  return dueDate < localDateKey(now);
}

const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

/** `2026-11-05` → "Nov 5"; adds the year when it isn't the current one ("Jan 3, 2027"). */
export function formatDueDate(dueDate: string | null | undefined, now: Date = new Date()): string {
  if (!dueDate) return "";
  const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(dueDate);
  if (!m) return dueDate;
  const [, y, mo, d] = m;
  const base = `${MONTHS[Number(mo) - 1]} ${Number(d)}`;
  return Number(y) === now.getFullYear() ? base : `${base}, ${y}`;
}

/** Absolute local date/time for tooltips. */
export function formatDateTime(iso: string | null | undefined): string {
  if (!iso) return "";
  const t = new Date(iso);
  return Number.isNaN(t.getTime()) ? "" : t.toLocaleString();
}
