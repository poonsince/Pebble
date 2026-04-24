import { describe, expect, it } from "vitest";
import type { Folder } from "../../src/lib/api";
import {
  allAccountsFolderId,
  buildAllAccountsFolders,
  folderIdsForSelection,
  roleFromAllAccountsFolderId,
  unreadCountForFolder,
} from "../../src/lib/folderAggregation";

function folder(id: string, accountId: string, role: Folder["role"], name = id): Folder {
  return {
    id,
    account_id: accountId,
    remote_id: id,
    name,
    folder_type: "folder",
    role,
    parent_id: null,
    color: null,
    is_system: true,
    sort_order: 0,
  };
}

describe("folder aggregation", () => {
  it("builds virtual all-account folders for shared system roles", () => {
    const folders = [
      folder("a1-inbox", "a1", "inbox"),
      folder("a2-inbox", "a2", "inbox"),
      folder("a2-sent", "a2", "sent"),
      folder("a1-project", "a1", null, "Project"),
    ];

    const aggregated = buildAllAccountsFolders(folders);

    expect(aggregated.map((f) => f.id)).toEqual([
      "all:inbox",
      "all:sent",
      "a1-project",
    ]);
    expect(aggregated[0]).toMatchObject({
      account_id: "all",
      role: "inbox",
      name: "Inbox",
    });
  });

  it("resolves virtual folder selections to all matching real folder ids", () => {
    const folders = [
      folder("a1-inbox", "a1", "inbox"),
      folder("a2-inbox", "a2", "inbox"),
      folder("a1-sent", "a1", "sent"),
    ];

    expect(folderIdsForSelection(allAccountsFolderId("inbox"), folders)).toEqual([
      "a1-inbox",
      "a2-inbox",
    ]);
    expect(roleFromAllAccountsFolderId("all:inbox")).toBe("inbox");
  });

  it("sums unread counts for virtual role folders", () => {
    const folders = [
      folder("a1-inbox", "a1", "inbox"),
      folder("a2-inbox", "a2", "inbox"),
      folder("a1-sent", "a1", "sent"),
    ];

    expect(
      unreadCountForFolder(allAccountsFolderId("inbox"), folders, {
        "a1-inbox": 2,
        "a2-inbox": 3,
        "a1-sent": 7,
      }),
    ).toBe(5);
  });
});
