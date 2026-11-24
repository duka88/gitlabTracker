// Client-side row filters for the list view (pure; tested in __tests__/filter.test.ts).
import type { ListItem } from "../types";

export interface RowFilters {
  /** Only MRs that aren't approved yet. Work-item rows have no approval and are hidden. */
  notApproved: boolean;
  /** Only rows not deployed on any tracked env; "outdated" there counts as not deployed. */
  notDeployed: boolean;
}

export const NO_FILTERS: RowFilters = { notApproved: false, notDeployed: false };

export const isDeployed = (item: ListItem) => (item.envs ?? []).some((e) => e.upToDate);

/** Rows matching every ticked filter, in their original order. */
export function filterItems<T extends ListItem>(items: readonly T[], f: RowFilters): readonly T[] {
  if (!f.notApproved && !f.notDeployed) return items;
  return items.filter(
    (i) => (!f.notApproved || (i.mr !== undefined && !i.mr.approvals.approved)) && (!f.notDeployed || !isDeployed(i)),
  );
}
