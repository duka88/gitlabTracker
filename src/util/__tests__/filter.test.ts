import { describe, expect, it } from "vitest";
import { NO_FILTERS, filterItems } from "../filter";
import type { EnvBadge, ListItem, MrFields } from "../../types";

function row(n: number, approved: boolean | null, envs: EnvBadge[] = [], onBranch: boolean | null = null): ListItem {
  const mr =
    approved === null
      ? undefined
      : ({
          approvals: { approved, required: 1, given: approved ? 1 : 0 },
          onBranch: onBranch === null ? null : { branch: "resource-test", upToDate: onBranch, mergedAt: null },
        } as MrFields);
  return {
    gid: `g${n}`, kind: mr ? "mergeRequests" : "workItems", iid: String(n), reference: `!${n}`, title: "",
    webUrl: "", projectPath: "g/p", author: null, createdAt: "", updatedAt: "", labels: [], assignees: [],
    milestone: null, commentCount: 0, upvotes: 0, downvotes: 0, descriptionPreview: "", mr, envs,
  };
}

const iids = (items: readonly ListItem[]) => items.map((i) => i.iid).join(",");
const onQa1 = [{ env: "qa-1", upToDate: true }];
const outdated = [{ env: "qa-1", upToDate: false }];

describe("filterItems", () => {
  const items = [
    row(1, true, onQa1, true), // approved, deployed, on branch
    row(2, false, onQa1, true), // not approved
    row(3, true, [], true), // not deployed
    row(4, true, outdated, true), // outdated on qa-1
    row(5, true, onQa1, false), // outdated on the test branch
    row(6, true, onQa1), // not on the test branch
    row(7, null), // work item, nothing deployed
  ];

  it("returns the same list with no filters", () => {
    expect(filterItems(items, NO_FILTERS)).toBe(items);
  });

  it("filters one at a time", () => {
    expect(iids(filterItems(items, { ...NO_FILTERS, notApproved: true }))).toBe("2");
    expect(iids(filterItems(items, { ...NO_FILTERS, notDeployed: true }))).toBe("3,7");
    expect(iids(filterItems(items, { ...NO_FILTERS, notOnTestBranch: true }))).toBe("6");
    expect(iids(filterItems(items, { ...NO_FILTERS, outdated: true }))).toBe("4,5");
  });

  it("combines filters with OR", () => {
    expect(iids(filterItems(items, { ...NO_FILTERS, notApproved: true, outdated: true }))).toBe("2,4,5");
    expect(iids(filterItems(items, { notApproved: true, notDeployed: true, notOnTestBranch: true, outdated: true }))).toBe(
      "2,3,4,5,6,7",
    );
  });
});
