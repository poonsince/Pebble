import { renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useNotificationOpenNavigation } from "../../src/app/useNotificationOpenNavigation";

const mocks = vi.hoisted(() => ({
  listeners: new Map<string, (event: { payload: { message_id?: string } }) => void>(),
  openMessageInInbox: vi.fn(),
  invalidateQueries: vi.fn(),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn((eventName: string, handler: (event: { payload: { message_id?: string } }) => void) => {
    mocks.listeners.set(eventName, handler);
    return Promise.resolve(vi.fn());
  }),
}));

vi.mock("@tanstack/react-query", () => ({
  useQueryClient: () => ({
    invalidateQueries: mocks.invalidateQueries,
  }),
}));

vi.mock("../../src/stores/ui.store", () => ({
  useUIStore: (selector: (state: { openMessageInInbox: (messageId: string) => void }) => unknown) =>
    selector({ openMessageInInbox: mocks.openMessageInInbox }),
}));

describe("useNotificationOpenNavigation", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.listeners.clear();
  });

  it("opens the target inbox message when a notification is clicked", async () => {
    renderHook(() => useNotificationOpenNavigation());

    await waitFor(() => expect(mocks.listeners.has("mail:notification-open")).toBe(true));

    mocks.listeners.get("mail:notification-open")?.({
      payload: { message_id: "message-1" },
    });

    expect(mocks.openMessageInInbox).toHaveBeenCalledWith("message-1");
    expect(mocks.invalidateQueries).toHaveBeenCalledWith({ queryKey: ["messages"] });
    expect(mocks.invalidateQueries).toHaveBeenCalledWith({ queryKey: ["threads"] });
  });
});
