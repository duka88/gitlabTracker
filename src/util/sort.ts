// Client-side row ordering for a tab (pure; tested in __tests__/sort.test.ts).
import type { ListItem, TabSort } from "../types";

export const SORT_OPTIONS: { value: TabSort; label: string }[] = [
  { value: "gitlab", label: "GitLab order" },
  { value: "priority", label: "Priority" },
  { value: "stage", label: "Stage (dev → testing)" },
  { value: "label", label: "Label" },
  { value: "updated", label: "Last updated" },
  { value: "created", label: "Created" },
  { value: "title", label: "Title" },
];

const collator = new Intl.Collator(undefined, { sensitivity: "base", numeric: true });

/** A row's label titles, A–Z. */
const labelKey = (item: ListItem) => item.labels.map((l) => l.title).sort(collator.compare);

/** Rows without labels go last; otherwise compares the A–Z label lists title by title. */
function compareLabels(a: string[], b: string[]): number {
  if (!a.length || !b.length) return Number(!a.length) - Number(!b.length);
  for (let i = 0; i < Math.min(a.length, b.length); i++) {
    const c = collator.compare(a[i], b[i]);
    if (c) return c;
  }
  return a.length - b.length;
}

/** Workflow order of `state::` labels; other `state::` values follow, A–Z. */
export const STAGE_ORDER = ["in development", "ready for testing", "in testing", "ready to merge"];

const scoped = (item: ListItem, scope: string) =>
  item.labels.find((l) => l.title.toLowerCase().startsWith(`${scope}::`))?.title.slice(scope.length + 2) ?? null;

/** Value of the row's `priority::` label (`P1`), or null. */
const priorityOf = (item: ListItem) => scoped(item, "priority");

/** Value of the row's `state::` label; MR rows fall back to their linked ticket's state label. */
function stageOf(item: ListItem): string | null {
  return scoped(item, "state") ?? item.mr?.ticket?.stateLabel?.value ?? null;
}

/** Nulls last, then `P1` < `P2` < `P10`. */
function comparePriority(a: string | null, b: string | null): number {
  if (a === null || b === null) return Number(a === null) - Number(b === null);
  return collator.compare(a, b);
}

/** Nulls last, known stages in workflow order, then unknown stages A–Z. */
function compareStage(a: string | null, b: string | null): number {
  if (a === null || b === null) return Number(a === null) - Number(b === null);
  const rank = (s: string) => {
    const i = STAGE_ORDER.indexOf(s.trim().toLowerCase());
    return i < 0 ? STAGE_ORDER.length : i;
  };
  return rank(a) - rank(b) || collator.compare(a, b);
}

const time = (iso: string) => {
  const t = Date.parse(iso);
  return Number.isNaN(t) ? -Infinity : t;
};

/** A new array in the given order; ties keep GitLab's order. `gitlab` returns the input as is. */
export function sortItems(items: readonly ListItem[], sort: TabSort | undefined): readonly ListItem[] {
  switch (sort) {
    case "label": {
      const keys = new Map(items.map((i) => [i.gid, labelKey(i)]));
      return [...items].sort((a, b) => compareLabels(keys.get(a.gid)!, keys.get(b.gid)!));
    }
    case "updated":
      return [...items].sort((a, b) => time(b.updatedAt) - time(a.updatedAt));
    case "created":
      return [...items].sort((a, b) => time(b.createdAt) - time(a.createdAt));
    case "priority":
      return [...items].sort(
        (a, b) => comparePriority(priorityOf(a), priorityOf(b)) || compareStage(stageOf(a), stageOf(b)),
      );
    case "stage":
      return [...items].sort(
        (a, b) => compareStage(stageOf(a), stageOf(b)) || comparePriority(priorityOf(a), priorityOf(b)),
      );
    case "title":
      return [...items].sort((a, b) => collator.compare(a.title, b.title));
    default:
      return items;
  }
}
