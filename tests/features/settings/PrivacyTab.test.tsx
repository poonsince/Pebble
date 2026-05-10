import { render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import PrivacyTab from "../../../src/features/settings/PrivacyTab";

const mocks = vi.hoisted(() => ({
  activeAccountId: null as string | null,
  listUntrustedSenders: vi.fn(),
  removeUntrustedSender: vi.fn(),
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (_key: string, fallback?: string) => fallback ?? _key,
  }),
}));

vi.mock("../../../src/stores/mail.store", () => ({
  useMailStore: (selector: (state: { activeAccountId: string | null }) => unknown) =>
    selector({ activeAccountId: mocks.activeAccountId }),
}));

vi.mock("../../../src/stores/toast.store", () => ({
  useToastStore: {
    getState: () => ({ addToast: vi.fn() }),
  },
}));

vi.mock("../../../src/lib/api", () => ({
  listUntrustedSenders: mocks.listUntrustedSenders,
  removeUntrustedSender: mocks.removeUntrustedSender,
}));

describe("PrivacyTab", () => {
  beforeEach(() => {
    mocks.activeAccountId = null;
    mocks.listUntrustedSenders.mockReset();
    mocks.removeUntrustedSender.mockReset();
  });

  it("selects relaxed as the default privacy mode when there is no stored preference", () => {
    localStorage.removeItem("pebble-privacy-mode");

    render(<PrivacyTab />);

    expect(screen.getByText("Same as Strict. Images load freely; only trackers from untrusted senders are blocked.")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Normal" }).getAttribute("style")).toContain(
      "var(--color-accent)",
    );
  });

  it("clears untrusted senders when there is no active account", async () => {
    mocks.activeAccountId = "account-1";
    mocks.listUntrustedSenders.mockResolvedValue([
      {
        account_id: "account-1",
        email: "spammy@example.com",
        created_at: 1,
      },
    ]);

    const { rerender } = render(<PrivacyTab />);

    expect(await screen.findByText("spammy@example.com")).toBeTruthy();

    mocks.activeAccountId = null;
    rerender(<PrivacyTab />);

    await waitFor(() => {
      expect(screen.queryByText("spammy@example.com")).toBeNull();
    });
    expect(screen.getByText("No untrusted senders. By default, all senders are trusted and no tracker blocking is applied. Tap the banner on a message to block a sender's trackers.")).toBeTruthy();
  });
});
