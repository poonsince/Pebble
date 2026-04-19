import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { useQueryClient } from "@tanstack/react-query";
import { useUIStore } from "../stores/ui.store";

interface NotificationOpenPayload {
  message_id?: string;
}

export function useNotificationOpenNavigation() {
  const openMessageInInbox = useUIStore((s) => s.openMessageInInbox);
  const queryClient = useQueryClient();

  useEffect(() => {
    const unlisten = listen<NotificationOpenPayload>("mail:notification-open", (event) => {
      const messageId = event.payload.message_id;
      if (!messageId) return;

      queryClient.invalidateQueries({ queryKey: ["messages"] });
      queryClient.invalidateQueries({ queryKey: ["threads"] });
      openMessageInInbox(messageId);
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [openMessageInInbox, queryClient]);
}
