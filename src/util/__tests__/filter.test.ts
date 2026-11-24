import { describe, expect, it } from "vitest";
import { NO_FILTERS, filterItems } from "../filter";
import type { EnvBadge, ListItem, MrFields } from "../../types";

function row(n: number, approved: boolean | null, envs: EnvBadge[] = []): ListItem {
  const mr = approved === null ? undefined : ({ approvals: { approved, required: 1, given: approved ? 1 : 0 } } as MrFields);
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
  const items = [row(1, true, onQa1), row(2, false, onQa1), row(3, false), row(4, true, outdated), row(5, null)];

  it("returns the same list with no filters", () => {
    expect(filterItems(items, NO_FILTERS)).toBe(items);
  });

  it("keeps only MRs that aren't approved", () => {
    expect(iids(filterItems(items, { notApproved: true, notDeployed: false }))).toBe("2,3");
  });

  it("keeps rows not deployed anywhere, counting outdated as not deployed", () => {
    expect(iids(filterItems(items, { notApproved: false, notDeployed: true }))).toBe("3,4,5");
  });

  it("combines both filters", () => {
    expect(iids(filterItems(items, { notApproved: true, notDeployed: true }))).toBe("3");
  });
});
