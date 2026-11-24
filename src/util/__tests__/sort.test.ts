import { describe, expect, it } from "vitest";
import { sortItems } from "../sort";
import type { Label, ListItem } from "../../types";

const label = (title: string): Label => ({ title, color: "#fff", textColor: "#000", scope: null, value: title });

function item(n: number, opts: Partial<ListItem> & { labelTitles?: string[] } = {}): ListItem {
  const { labelTitles = [], ...rest } = opts;
  return {
    gid: `gid://gitlab/WorkItem/${n}`, kind: "workItems", iid: String(n), reference: `#${n}`,
    title: `T${n}`, webUrl: "", projectPath: "g/p", author: null, createdAt: "2026-01-01T00:00:00Z",
    updatedAt: "2026-01-01T00:00:00Z", closedAt: null, labels: labelTitles.map(label), assignees: [],
    milestone: null, commentCount: 0, upvotes: 0, downvotes: 0, descriptionPreview: "", ...rest,
  };
}

const iids = (items: readonly ListItem[]) => items.map((i) => i.iid).join(",");

describe("sortItems", () => {
  it("keeps GitLab order by default", () => {
    const items = [item(2), item(1)];
    expect(sortItems(items, "gitlab")).toBe(items);
    expect(sortItems(items, undefined)).toBe(items);
  });

  it("sorts by label A–Z, unlabelled last, ties in GitLab order", () => {
    const items = [
      item(1),
      item(2, { labelTitles: ["state::Review", "bug"] }), // key: bug, state::Review
      item(3, { labelTitles: ["Backend"] }),
      item(4, { labelTitles: ["bug"] }), // key: bug (shorter → before 2)
      item(5, { labelTitles: ["Backend"] }),
      item(6),
    ];
    expect(iids(sortItems(items, "label"))).toBe("3,5,4,2,1,6");
  });

  it("sorts by dates newest first and by title", () => {
    const items = [
      item(1, { updatedAt: "2026-03-01T00:00:00Z", createdAt: "2026-01-02T00:00:00Z", title: "beta" }),
      item(2, { updatedAt: "2026-05-01T00:00:00Z", createdAt: "2026-01-01T00:00:00Z", title: "Alpha" }),
      item(3, { updatedAt: "2026-04-01T00:00:00Z", createdAt: "2026-01-03T00:00:00Z", title: "item 10" }),
      item(4, { updatedAt: "bad", createdAt: "bad", title: "item 9" }),
    ];
    expect(iids(sortItems(items, "updated"))).toBe("2,3,1,4");
    expect(iids(sortItems(items, "created"))).toBe("3,1,2,4");
    expect(iids(sortItems(items, "title"))).toBe("2,1,4,3");
  });

  it("sorts by priority (P1 first, none last), ties by stage", () => {
    const items = [
      item(1, { labelTitles: ["priority::P3"] }),
      item(2),
      item(3, { labelTitles: ["priority::P1", "state::In Testing"] }),
      item(4, { labelTitles: ["Priority::P1", "state::In Development"] }),
      item(5, { labelTitles: ["priority::P10"] }),
      item(6, { labelTitles: ["priority::P2"] }),
    ];
    expect(iids(sortItems(items, "priority"))).toBe("4,3,6,1,5,2");
  });

  it("sorts by stage in workflow order, ties by priority, MRs use the linked ticket", () => {
    const mr: ListItem = {
      ...item(7),
      kind: "mergeRequests",
      mr: { ticket: { stateLabel: { ...label("state::Ready for Testing"), value: "Ready for Testing" } } } as ListItem["mr"],
    };
    const items = [
      item(1, { labelTitles: ["state::Ready to Merge"] }),
      item(2, { labelTitles: ["state::In Testing", "priority::P3"] }),
      item(3),
      item(4, { labelTitles: ["state::In Development"] }),
      item(5, { labelTitles: ["state::Blocked"] }),
      item(6, { labelTitles: ["state::In Testing", "priority::P1"] }),
      mr,
    ];
    expect(iids(sortItems(items, "stage"))).toBe("4,7,6,2,1,5,3");
  });

  it("sorts by approval: approved first, then fewest needed, then most given, no MR last", () => {
    const mr = (n: number, approved: boolean, required: number, given: number): ListItem => ({
      ...item(n),
      kind: "mergeRequests",
      mr: { approvals: { approved, required, given } } as ListItem["mr"],
    });
    const items = [item(1), mr(2, false, 2, 0), mr(3, true, 1, 1), mr(4, false, 2, 1), mr(5, true, 0, 2), mr(6, false, 0, 0)];
    expect(iids(sortItems(items, "approval"))).toBe("5,3,6,4,2,1");
  });
});
