import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useUIStore } from "../../src/stores/ui.store";

describe("UIStore", () => {
  beforeEach(() => {
    useUIStore.setState({
      sidebarCollapsed: false,
      activeView: "inbox",
      theme: "light",
      language: "en",
      syncStatus: "idle",
      networkStatus: "online",
      lastMailError: null,
      previousView: "inbox",
      composeMode: null,
      composeReplyTo: null,
      composeDirty: false,
      showComposeLeaveConfirm: false,
      pendingView: null,
      pollInterval: 15,
      searchQuery: "",
    });
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("should have correct initial state", () => {
    const state = useUIStore.getState();
    expect(state.sidebarCollapsed).toBe(false);
    expect(state.activeView).toBe("inbox");
    expect(state.theme).toBe("light");
    expect(state.syncStatus).toBe("idle");
  });

  it("should toggle sidebar", () => {
    useUIStore.getState().toggleSidebar();
    expect(useUIStore.getState().sidebarCollapsed).toBe(true);
    useUIStore.getState().toggleSidebar();
    expect(useUIStore.getState().sidebarCollapsed).toBe(false);
  });

  it("should set active view", () => {
    useUIStore.getState().setActiveView("kanban");
    expect(useUIStore.getState().activeView).toBe("kanban");
    useUIStore.getState().setActiveView("settings");
    expect(useUIStore.getState().activeView).toBe("settings");
  });

  it("keeps the user on compose when dirty and shows confirmation", () => {
    useUIStore.setState({
      activeView: "compose",
      previousView: "inbox",
      composeMode: "new",
      composeDirty: true,
    });

    useUIStore.getState().setActiveView("search");

    const state = useUIStore.getState();
    // Should stay on compose and show confirmation dialog
    expect(state.activeView).toBe("compose");
    expect(state.composeMode).toBe("new");
    expect(state.composeDirty).toBe(true);
    expect(state.showComposeLeaveConfirm).toBe(true);
    expect(state.pendingView).toBe("search");
  });

  it("closeCompose respects unsaved-draft protection", () => {
    useUIStore.setState({
      activeView: "compose",
      previousView: "kanban",
      composeMode: "reply",
      composeDirty: true,
    });

    useUIStore.getState().closeCompose();

    const state = useUIStore.getState();
    // Should stay on compose and show confirmation dialog
    expect(state.activeView).toBe("compose");
    expect(state.composeMode).toBe("reply");
    expect(state.composeDirty).toBe(true);
    expect(state.showComposeLeaveConfirm).toBe(true);
  });

  it("confirmCloseCompose navigates away and clears compose state", () => {
    useUIStore.setState({
      activeView: "compose",
      previousView: "kanban",
      composeMode: "forward",
      composeReplyTo: { id: "message-1" } as never,
      composeDirty: true,
      showComposeLeaveConfirm: true,
      pendingView: null,
    });

    useUIStore.getState().confirmCloseCompose();

    const state = useUIStore.getState();
    expect(state.activeView).toBe("kanban");
    expect(state.composeMode).toBe(null);
    expect(state.composeReplyTo).toBe(null);
    expect(state.composeDirty).toBe(false);
    expect(state.showComposeLeaveConfirm).toBe(false);
  });

  it("confirmCloseCompose navigates to pendingView when set", () => {
    useUIStore.setState({
      activeView: "compose",
      previousView: "inbox",
      composeMode: "new",
      composeDirty: true,
      showComposeLeaveConfirm: true,
      pendingView: "search",
    });

    useUIStore.getState().confirmCloseCompose();

    const state = useUIStore.getState();
    expect(state.activeView).toBe("search");
    expect(state.composeMode).toBe(null);
    expect(state.showComposeLeaveConfirm).toBe(false);
    expect(state.pendingView).toBe(null);
  });

  it("cancelCloseCompose clears confirmation state", () => {
    useUIStore.setState({
      activeView: "compose",
      previousView: "inbox",
      composeMode: "new",
      composeDirty: true,
      showComposeLeaveConfirm: true,
      pendingView: "kanban",
    });

    useUIStore.getState().cancelCloseCompose();

    const state = useUIStore.getState();
    expect(state.activeView).toBe("compose");
    expect(state.composeMode).toBe("new");
    expect(state.composeDirty).toBe(true);
    expect(state.showComposeLeaveConfirm).toBe(false);
    expect(state.pendingView).toBe(null);
  });

  it("should set theme", () => {
    useUIStore.getState().setTheme("dark");
    expect(useUIStore.getState().theme).toBe("dark");
  });

  it("should set sync status", () => {
    useUIStore.getState().setSyncStatus("syncing");
    expect(useUIStore.getState().syncStatus).toBe("syncing");
    useUIStore.getState().setSyncStatus("error");
    expect(useUIStore.getState().syncStatus).toBe("error");
  });
});
