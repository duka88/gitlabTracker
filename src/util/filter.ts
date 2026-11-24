// Client-side row filters for the list view (pure; tested in __tests__/filter.test.ts).
import type { ListItem } from "../types";

export interface RowFilters {
  /** MRs that aren't approved yet. */
  notApproved: boolean;
  /** Rows not on any tracked env at all (no env chip, FR-040). */
  notDeployed: boolean;
  /** MRs not merged into the test branch at all (no "on" / "outdated on" tag, FR-037). */
  notOnTestBranch: boolean;
  /** Rows that are on the test branch or an env, but with newer commits not there yet. */
  outdated: boolean;
}

export const NO_FILTERS: RowFilters = { notApproved: false, notDeployed: false, notOnTestBranch: false, outdated: false };

export const isOutdated = (item: ListItem) =>
  item.mr?.onBranch?.upToDate === false || (item.envs ?? []).some((e) => !e.upToDate);

/** Rows matching at least one ticked filter (OR), in their original order. */
export function filterItems<T extends ListItem>(items: readonly T[], f: RowFilters): readonly T[] {
  if (!f.notApproved && !f.notDeployed && !f.notOnTestBranch && !f.outdated) return items;
  return items.filter(
    (i) =>
      (f.notApproved && i.mr !== undefined && !i.mr.approvals.approved) ||
      (f.notDeployed && (i.envs ?? []).length === 0) ||
      (f.notOnTestBranch && i.mr !== undefined && !i.mr.onBranch) ||
      (f.outdated && isOutdated(i)),
  );
}
