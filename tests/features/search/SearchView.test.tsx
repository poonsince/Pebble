import { act, render, screen, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { SearchHit } from "../../../src/lib/api";
import SearchView from "../../../src/features/search/SearchView";
import { useUIStore } from "../../../src/stores/ui.store";

let searchResults: SearchHit[] = [];

vi.mock("react-i18next", () => ({
  initReactI18next: {
    type: "3rdParty",
    init: vi.fn(),
  },
  useTranslation: () => ({
    t: (key: string, fallback?: string) => {
      const labels: Record<string, string> = {
        "inbox.searchPlaceholder": "Search mail",
        "search.title": "Search",
        "search.searchButton": "Search",
        "search.filters": "Filters",
        "search.results": "Search results",
      };
      return labels[key] ?? fallback ?? key;
    },
  }),
}));

vi.mock("@tanstack/react-query", () => ({
  useQuery: () => ({
    data: searchResults,
    isLoading: false,
    error: null,
    refetch: vi.fn(),
  }),
}));

vi.mock("@tanstack/react-virtual", () => ({
  useVirtualizer: ({ count }: { count: number }) => ({
    getTotalSize: () => count * 76,
    getVirtualItems: () =>
      Array.from({ length: count }, (_, index) => ({
        index,
        key: `row-${index}`,
        start: index * 76,
      })),
    measureElement: vi.fn(),
  }),
}));

vi.mock("../../../src/features/search/SearchFilters", () => ({
  default: () => <div>Search filters</div>,
}));

vi.mock("../../../src/features/search/SearchResultItem", () => ({
  default: ({ hit, isSelected }: { hit: SearchHit; isSelected: boolean }) => (
    <div role="option" aria-selected={isSelected}>
      {hit.subject}
    </div>
  ),
}));

vi.mock("../../../src/components/MessageDetail", () => ({
  default: () => <div>Message detail</div>,
}));

describe("SearchView", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    searchResults = [];
    useUIStore.setState({ activeView: "search", searchQuery: "" });
  });

  it("picks up context search queries while already mounted", () => {
    render(<SearchView />);

    act(() => {
      useUIStore.getState().setSearchQuery("invoice total");
    });

    expect(screen.getByRole<HTMLInputElement>("textbox", { name: "Search" }).value).toBe("invoice total");
    expect(useUIStore.getState().searchQuery).toBe("");
  });

  it("groups virtualized search results in a named listbox", () => {
    searchResults = [{
      message_id: "message-1",
      score: 1,
      subject: "Invoice total",
      snippet: "The invoice total is ready",
      from_address: "sender@example.com",
      date: 1_700_000_000,
    }];

    render(<SearchView />);

    const listbox = screen.getByRole("listbox", { name: "Search results" });
    expect(within(listbox).getByRole("option", { name: "Invoice total" })).toBeTruthy();
  });
});
