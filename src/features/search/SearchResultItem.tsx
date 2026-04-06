import { useTranslation } from "react-i18next";
import type { SearchHit, Message } from "@/lib/api";

interface Props {
  hit: SearchHit;
  message?: Message | null;
  isSelected: boolean;
  onClick: () => void;
}

export default function SearchResultItem({ hit, message, isSelected, onClick }: Props) {
  const { t } = useTranslation();
  const subject = hit.subject || message?.subject || hit.snippet || t("common.noSubject");
  const from = hit.from_address || message?.from_address || "";
  const date = (hit.date || message?.date) ? new Date((hit.date || message!.date) * 1000).toLocaleDateString() : "";

  return (
    <div
      role="option"
      aria-selected={isSelected}
      tabIndex={0}
      onClick={onClick}
      onKeyDown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); onClick(); } }}
      style={{
        padding: "10px 14px",
        cursor: "pointer",
        borderBottom: "1px solid var(--color-border)",
        backgroundColor: isSelected ? "var(--color-bg-hover)" : "transparent",
      }}
      onMouseEnter={(e) => {
        if (!isSelected) e.currentTarget.style.backgroundColor = "var(--color-bg-hover)";
      }}
      onMouseLeave={(e) => {
        if (!isSelected) e.currentTarget.style.backgroundColor = "transparent";
      }}
    >
      <div
        style={{
          fontSize: "13px",
          fontWeight: 600,
          color: "var(--color-text-primary)",
          marginBottom: "2px",
          overflow: "hidden",
          textOverflow: "ellipsis",
          whiteSpace: "nowrap",
        }}
      >
        {subject}
      </div>
      <div
        style={{
          fontSize: "12px",
          color: "var(--color-text-secondary)",
          marginBottom: "2px",
          overflow: "hidden",
          textOverflow: "ellipsis",
          whiteSpace: "nowrap",
        }}
      >
        {from}
      </div>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
        <div
          style={{
            fontSize: "12px",
            color: "var(--color-text-tertiary)",
            flex: 1,
            overflow: "hidden",
            textOverflow: "ellipsis",
            whiteSpace: "nowrap",
            marginRight: "8px",
          }}
        >
          {hit.snippet}
        </div>
        {date && (
          <div style={{ fontSize: "11px", color: "var(--color-text-tertiary)", flexShrink: 0 }}>
            {date}
          </div>
        )}
      </div>
    </div>
  );
}
