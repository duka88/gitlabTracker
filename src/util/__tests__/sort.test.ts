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
});
