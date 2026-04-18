import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { MessageSummary } from "../../src/lib/api";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, fallback?: string) => {
      const labels: Record<string, string> = {
        "messageActions.archive": "Archive",
        "messageActions.unarchive": "Unarchive",
        "messageActions.addToKanban": "Add to kanban",
        "messageActions.star": "Star",
        "messageActions.unstar": "Unstar",
      };
      return labels[key] ?? fallback ?? key;
    },
  }),
}));

vi.mock("@tanstack/react-query", () => ({
  useQueryClient: () => ({
    invalidateQueries: vi.fn(),
  }),
}));

vi.mock("../../src/hooks/queries", () => ({
  patchMessagesCache: vi.fn(),
}));

vi.mock("../../src/lib/api", () => ({
  updateMessageFlags: vi.fn(),
  archiveMessage: vi.fn(),
  moveToFolder: vi.fn(),
}));

vi.mock("../../src/stores/kanban.store", () => ({
  useKanbanStore: (selector: (state: { cardIdSet: Set<string> }) => unknown) =>
    selector({ cardIdSet: new Set() }),
}));

vi.mock("../../src/stores/toast.store", () => ({
  useToastStore: {
    getState: () => ({ addToast: vi.fn() }),
  },
}));

import MessageItem from "../../src/components/MessageItem";

function makeMessage(): MessageSummary {
  return {
    id: "message-1",
    account_id: "account-1",
    remote_id: "remote-message-1",
    message_id_header: null,
    in_reply_to: null,
    references_header: null,
    thread_id: null,
    subject: "Archived message",
    snippet: "Snippet",
    from_address: "sender@example.com",
    from_name: "Sender",
    to_list: [],
    cc_list: [],
    bcc_list: [],
    has_attachments: false,
    is_read: true,
    is_starred: false,
    is_draft: false,
    date: 1_700_000_000,
    remote_version: null,
    is_deleted: false,
    deleted_at: null,
    created_at: 1_700_000_000,
    updated_at: 1_700_000_000,
  };
}

describe("MessageItem", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("labels the archive action as unarchive in the archive folder", () => {
    render(
      <MessageItem
        message={makeMessage()}
        isSelected={false}
        onClick={vi.fn()}
        {...({ folderRole: "archive" } as Record<string, unknown>)}
      />,
    );

    fireEvent.mouseEnter(screen.getByRole("option"));

    expect(screen.getByRole("button", { name: "Unarchive" })).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Archive" })).toBeNull();
  });
});
