// Client-side row ordering for a tab (pure; tested in __tests__/sort.test.ts).
import type { ListItem, TabSort } from "../types";

export const SORT_OPTIONS: { value: TabSort; label: string }[] = [
  { value: "gitlab", label: "GitLab order" },
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
    case "title":
      return [...items].sort((a, b) => collator.compare(a.title, b.title));
    default:
      return items;
  }
}
