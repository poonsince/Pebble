import { useQuery } from "@tanstack/react-query";
import { getFolderUnreadCounts } from "@/lib/api";
import { useUIStore } from "@/stores/ui.store";

export function useFolderUnreadCounts(accountId: string | null) {
  const enabled = useUIStore((s) => s.showFolderUnreadCount);
  return useQuery({
    queryKey: ["folder-unread-counts", accountId],
    queryFn: () => getFolderUnreadCounts(accountId!),
    enabled: enabled && !!accountId,
    staleTime: 30_000,
    refetchInterval: 30_000,
  });
}
