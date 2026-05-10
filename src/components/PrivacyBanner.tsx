import { Shield } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { RenderedHtml } from "@/lib/api";

interface Props {
  rendered: RenderedHtml;
  onBlockSender: () => void;
}

export default function PrivacyBanner({ rendered, onBlockSender }: Props) {
  const { t } = useTranslation();
  const totalBlocked = rendered.trackers_blocked.length;

  if (totalBlocked === 0) {
    return null;
  }

  const parts: string[] = [];
  if (rendered.trackers_blocked.length > 0) {
    parts.push(t("privacy.tracker", { count: rendered.trackers_blocked.length }));
  }

  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        gap: "10px",
        padding: "8px 14px",
        backgroundColor: "rgba(245, 158, 11, 0.08)",
        borderBottom: "1px solid var(--color-border)",
        fontSize: "12px",
        color: "var(--color-text-secondary)",
      }}
    >
      <Shield size={14} color="#f59e0b" />
      <span style={{ flex: 1 }}>
        {t("privacy.blocked", { items: parts.join(` ${t("privacy.and")} `) })}
      </span>
      <button
        onClick={onBlockSender}
        style={{
          fontSize: "12px",
          padding: "2px 8px",
          borderRadius: "4px",
          border: "1px solid var(--color-border)",
          backgroundColor: "transparent",
          color: "var(--color-text-primary)",
          cursor: "pointer",
        }}
      >
        {t("privacy.blockSender", "Block this sender's trackers")}
      </button>
    </div>
  );
}
