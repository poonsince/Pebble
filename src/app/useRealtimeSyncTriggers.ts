import { useEffect } from "react";
import { triggerSync } from "@/lib/api";
import { useMailStore } from "@/stores/mail.store";
import { useUIStore } from "@/stores/ui.store";

export function useRealtimeSyncTriggers() {
  const activeAccountId = useMailStore((s) => s.activeAccountId);
  const networkStatus = useUIStore((s) => s.networkStatus);

  useEffect(() => {
    if (!activeAccountId) return;

    const onFocus = () => {
      triggerSync(activeAccountId, "window_focus").catch(() => {});
    };

    window.addEventListener("focus", onFocus);
    return () => window.removeEventListener("focus", onFocus);
  }, [activeAccountId]);

  useEffect(() => {
    if (!activeAccountId || networkStatus !== "online") return;
    triggerSync(activeAccountId, "network_online").catch(() => {});
  }, [activeAccountId, networkStatus]);
}
