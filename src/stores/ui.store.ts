import { create } from "zustand";
import { safeGetItem, safeSetItem } from "@/lib/browserStorage";
import i18n from "@/lib/i18n";
import { getInitialLanguage, LANGUAGE_STORAGE_KEY, type Language } from "@/lib/language";
import { useComposeStore } from "./compose.store";
import { useMailStore } from "./mail.store";

export type ActiveView = "inbox" | "kanban" | "settings" | "search" | "snoozed" | "starred" | "compose";
export type SettingsTab = "accounts" | "general" | "proxy" | "appearance" | "privacy" | "rules" | "remoteWrites" | "translation" | "shortcuts" | "cloudSync" | "about" | "developer";
export type Theme = "light" | "dark" | "system";
export type { Language } from "@/lib/language";
export type NetworkStatus = "online" | "offline";
export type RealtimeMode = "realtime" | "polling" | "manual" | "backoff" | "offline" | "auth_required" | "error";
export type RealtimePreference = "realtime" | "balanced" | "battery" | "manual";

export interface RealtimeStatus {
  account_id: string;
  mode: RealtimeMode;
  provider: string;
  last_success_at?: number | null;
  next_retry_at?: number | null;
  message?: string | null;
}

const REALTIME_PREFERENCE_KEY = "pebble-realtime-mode";
const REALTIME_PREFERENCES = new Set<RealtimePreference>(["realtime", "balanced", "battery", "manual"]);
const NOTIFICATIONS_KEY = "pebble-notifications-enabled";
const KEEP_RUNNING_BACKGROUND_KEY = "pebble-keep-running-background";
const DEVTOOLS_AUTO_OPEN_KEY = "pebble-devtools-auto-open";

function readRealtimePreference(): RealtimePreference {
  const stored = safeGetItem(localStorage, REALTIME_PREFERENCE_KEY);
  return REALTIME_PREFERENCES.has(stored as RealtimePreference)
    ? (stored as RealtimePreference)
    : "realtime";
}

export function readNotificationsEnabledPreference(): boolean {
  const stored = safeGetItem(localStorage, NOTIFICATIONS_KEY);
  return stored === null ? true : stored === "true";
}

export function readKeepRunningInBackgroundPreference(): boolean {
  const stored = safeGetItem(localStorage, KEEP_RUNNING_BACKGROUND_KEY);
  return stored === null ? true : stored === "true";
}

export function readDevtoolsAutoOpenPreference(): boolean {
  return safeGetItem(localStorage, DEVTOOLS_AUTO_OPEN_KEY) === "true";
}

export function realtimePreferenceToPollInterval(mode: RealtimePreference): number {
  switch (mode) {
    case "realtime":
      return 3;
    case "balanced":
      return 15;
    case "battery":
      return 60;
    case "manual":
      return 0;
  }
}

const initialRealtimeMode = readRealtimePreference();
const initialNotificationsEnabled = readNotificationsEnabledPreference();
const initialKeepRunningInBackground = readKeepRunningInBackgroundPreference();
const initialDevtoolsAutoOpen = readDevtoolsAutoOpenPreference();
const initialLanguage = getInitialLanguage();

/** Resolve "system" theme to an actual "dark" | "light" value. */
function resolveTheme(theme: Theme): "dark" | "light" {
  if (theme === "system") {
    return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
  }
  return theme;
}

/** Apply the resolved theme to the DOM immediately (no React effect needed). */
export function applyThemeToDom(theme: Theme) {
  document.documentElement.setAttribute("data-theme", resolveTheme(theme));
}

interface UIState {
  sidebarCollapsed: boolean;
  activeView: ActiveView;
  theme: Theme;
  language: Language;
  syncStatus: "idle" | "syncing" | "error";
  networkStatus: NetworkStatus;
  lastMailError: string | null;
  realtimeStatusByAccount: Record<string, RealtimeStatus>;
  realtimeMode: RealtimePreference;
  notificationsEnabled: boolean;
  setNotificationsEnabled: (enabled: boolean) => void;
  keepRunningInBackground: boolean;
  setKeepRunningInBackground: (enabled: boolean) => void;
  devtoolsAutoOpen: boolean;
  setDevtoolsAutoOpen: (enabled: boolean) => void;
  previousView: ActiveView;
  toggleSidebar: () => void;
  setActiveView: (view: ActiveView) => void;
  openMessageInInbox: (messageId: string) => void;
  setTheme: (theme: Theme) => void;
  setLanguage: (lang: Language) => void;
  setSyncStatus: (status: "idle" | "syncing" | "error") => void;
  setNetworkStatus: (status: NetworkStatus) => void;
  setLastMailError: (error: string | null) => void;
  setRealtimeStatus: (accountId: string, status: RealtimeStatus) => void;
  setRealtimeMode: (mode: RealtimePreference) => void;
  pollInterval: number;
  setPollInterval: (secs: number) => void;
  searchQuery: string;
  setSearchQuery: (q: string) => void;
  settingsTab: SettingsTab;
  setSettingsTab: (tab: SettingsTab) => void;
  pendingRuleDraftText: string | null;
  setPendingRuleDraftText: (text: string | null) => void;
  showFolderUnreadCount: boolean;
  setShowFolderUnreadCount: (show: boolean) => void;
}

export const useUIStore = create<UIState>((set) => ({
  sidebarCollapsed: false,
  activeView: "inbox",
  theme: (safeGetItem(localStorage, "pebble-theme") as Theme) || "light",
  language: initialLanguage,
  syncStatus: "idle",
  networkStatus: "online",
  lastMailError: null,
  realtimeStatusByAccount: {},
  realtimeMode: initialRealtimeMode,
  notificationsEnabled: initialNotificationsEnabled,
  setNotificationsEnabled: (enabled) => {
    safeSetItem(localStorage, NOTIFICATIONS_KEY, String(enabled));
    set({ notificationsEnabled: enabled });
  },
  keepRunningInBackground: initialKeepRunningInBackground,
  setKeepRunningInBackground: (enabled) => {
    safeSetItem(localStorage, KEEP_RUNNING_BACKGROUND_KEY, String(enabled));
    set({ keepRunningInBackground: enabled });
  },
  devtoolsAutoOpen: initialDevtoolsAutoOpen,
  setDevtoolsAutoOpen: (enabled) => {
    safeSetItem(localStorage, DEVTOOLS_AUTO_OPEN_KEY, String(enabled));
    set({ devtoolsAutoOpen: enabled });
  },
  previousView: "inbox",
  toggleSidebar: () =>
    set((state) => ({ sidebarCollapsed: !state.sidebarCollapsed })),
  setActiveView: (view) => {
    const state = useUIStore.getState();
    if (state.activeView === view) {
      return;
    }

    // Delegate dirty-compose guard to the compose store
    if (state.activeView === "compose" && view !== "compose") {
      const composeState = useComposeStore.getState();
      if (composeState.composeDirty) {
        useComposeStore.setState({ showComposeLeaveConfirm: true, pendingView: view });
        return;
      }
      useComposeStore.setState({
        composeMode: null,
        composeReplyTo: null,
        composePrefill: null,
        composeDirty: false,
      });
      set({ activeView: view });
      return;
    }

    set({ activeView: view });
  },
  openMessageInInbox: (messageId) => {
    useMailStore.setState({
      selectedMessageId: messageId,
      selectedThreadId: null,
      threadView: false,
      selectedMessageIds: new Set(),
      batchMode: false,
    });
    set({ activeView: "inbox" });
  },
  setTheme: (theme) => {
    safeSetItem(localStorage, "pebble-theme", theme);
    applyThemeToDom(theme);
    set({ theme });
  },
  setLanguage: (lang) => {
    i18n.changeLanguage(lang);
    safeSetItem(localStorage, LANGUAGE_STORAGE_KEY, lang);
    set({ language: lang });
  },
  setSyncStatus: (status) => set({ syncStatus: status }),
  setNetworkStatus: (status) => set({ networkStatus: status }),
  setLastMailError: (error) => set({ lastMailError: error }),
  setRealtimeStatus: (accountId, status) =>
    set((state) => ({
      realtimeStatusByAccount: {
        ...state.realtimeStatusByAccount,
        [accountId]: status,
      },
    })),
  setRealtimeMode: (mode) => {
    const pollInterval = realtimePreferenceToPollInterval(mode);
    safeSetItem(localStorage, REALTIME_PREFERENCE_KEY, mode);
    safeSetItem(localStorage, "pebble-poll-interval", String(pollInterval));
    set({
      realtimeMode: mode,
      pollInterval,
    });
  },
  pollInterval: realtimePreferenceToPollInterval(initialRealtimeMode),
  setPollInterval: (secs) => {
    safeSetItem(localStorage, "pebble-poll-interval", String(secs));
    set({ pollInterval: secs });
  },
  searchQuery: "",
  setSearchQuery: (q) => set({ searchQuery: q }),
  settingsTab: (safeGetItem(sessionStorage, "pebble-settings-tab") as SettingsTab) || "accounts",
  setSettingsTab: (tab) => {
    safeSetItem(sessionStorage, "pebble-settings-tab", tab);
    set({ settingsTab: tab });
  },
  pendingRuleDraftText: null,
  setPendingRuleDraftText: (text) => set({ pendingRuleDraftText: text }),
  showFolderUnreadCount: safeGetItem(localStorage, "pebble-show-unread-count") === "true",
  setShowFolderUnreadCount: (show) => {
    safeSetItem(localStorage, "pebble-show-unread-count", String(show));
    set({ showFolderUnreadCount: show });
  },
}));
