import { useEffect } from "react";
import { setNotificationsEnabled, setRealtimePreference } from "@/lib/api";
import { useUIStore } from "@/stores/ui.store";

export function useRealtimePreferenceSync() {
  const realtimeMode = useUIStore((state) => state.realtimeMode);
  const notificationsEnabled = useUIStore((state) => state.notificationsEnabled);

  useEffect(() => {
    let cancelled = false;

    async function syncRealtimePreference() {
      try {
        await setNotificationsEnabled(notificationsEnabled);
      } catch {}

      if (cancelled) return;

      setRealtimePreference(realtimeMode).catch(() => {});
    }

    syncRealtimePreference();

    return () => {
      cancelled = true;
    };
  }, [realtimeMode]);
}
