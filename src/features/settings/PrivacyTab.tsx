import { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { useMailStore } from "@/stores/mail.store";
import { useToastStore } from "@/stores/toast.store";
import { listUntrustedSenders, removeUntrustedSender } from "@/lib/api";
import type { UntrustedSender } from "@/lib/api";
import {
  PRIVACY_MODE_KEY,
  readStoredPrivacyMode,
  type StoredPrivacyMode,
} from "@/lib/privacyMode";
import { Trash2 } from "lucide-react";

export default function PrivacyTab() {
  const { t } = useTranslation();
  const activeAccountId = useMailStore((s) => s.activeAccountId);
  const [untrustedSenders, setUntrustedSenders] = useState<UntrustedSender[]>([]);
  const [privacyMode, setPrivacyMode] = useState<StoredPrivacyMode>(() =>
    readStoredPrivacyMode(),
  );

  useEffect(() => {
    if (!activeAccountId) {
      setUntrustedSenders((prev) => prev.length === 0 ? prev : []);
      return;
    }

    let cancelled = false;
    listUntrustedSenders(activeAccountId)
      .then((senders) => {
        if (!cancelled) setUntrustedSenders(senders);
      })
      .catch((err) => {
        if (cancelled) return;
        console.warn("Failed to load untrusted senders", err);
        useToastStore.getState().addToast({
          message: t("privacy.loadUntrustedFailed", "Failed to load untrusted senders"),
          type: "error",
        });
      });

    return () => { cancelled = true; };
  }, [activeAccountId, t]);

  function handlePrivacyModeChange(mode: StoredPrivacyMode) {
    setPrivacyMode(mode);
    localStorage.setItem(PRIVACY_MODE_KEY, mode);
  }

  async function handleRemoveUntrusted(email: string) {
    if (!activeAccountId) return;
    try {
      await removeUntrustedSender(activeAccountId, email);
      setUntrustedSenders((prev) => prev.filter((s) => s.email !== email));
    } catch (err) {
      console.warn("Failed to remove untrusted sender", err);
      useToastStore.getState().addToast({
        message: t("privacy.removeUntrustedFailed", "Failed to remove untrusted sender"),
        type: "error",
      });
    }
  }

  return (
    <div>
      <h2 style={{ fontSize: "18px", fontWeight: 600, marginBottom: "20px" }}>
        {t("privacy.settingsTitle", "Privacy & Tracking")}
      </h2>

      {/* Global privacy mode */}
      <div style={{ marginBottom: "24px" }}>
        <label style={{ fontSize: "13px", fontWeight: 600, display: "block", marginBottom: "8px" }}>
          {t("privacy.defaultMode", "Default privacy mode")}
        </label>
        <div style={{ display: "flex", gap: "8px" }}>
          <button
            onClick={() => handlePrivacyModeChange("strict")}
            style={{
              padding: "8px 16px",
              borderRadius: "6px",
              border: "1px solid var(--color-border)",
              backgroundColor: privacyMode === "strict" ? "var(--color-accent)" : "var(--color-bg)",
              color: privacyMode === "strict" ? "#fff" : "var(--color-text-primary)",
              cursor: "pointer",
              fontSize: "13px",
            }}
          >
            {t("privacy.strict", "Strict")}
          </button>
          <button
            onClick={() => handlePrivacyModeChange("relaxed")}
            style={{
              padding: "8px 16px",
              borderRadius: "6px",
              border: "1px solid var(--color-border)",
              backgroundColor: privacyMode === "relaxed" ? "var(--color-accent)" : "var(--color-bg)",
              color: privacyMode === "relaxed" ? "#fff" : "var(--color-text-primary)",
              cursor: "pointer",
              fontSize: "13px",
            }}
          >
            {t("privacy.relaxed", "Normal")}
          </button>
          <button
            onClick={() => handlePrivacyModeChange("off")}
            style={{
              padding: "8px 16px",
              borderRadius: "6px",
              border: "1px solid var(--color-border)",
              backgroundColor: privacyMode === "off" ? "var(--color-accent)" : "var(--color-bg)",
              color: privacyMode === "off" ? "#fff" : "var(--color-text-primary)",
              cursor: "pointer",
              fontSize: "13px",
            }}
          >
            {t("privacy.off", "Off")}
          </button>
        </div>
        <p style={{ fontSize: "12px", color: "var(--color-text-secondary)", marginTop: "6px" }}>
          {privacyMode === "strict"
            ? t("privacy.strictDesc", "Images always load. Known tracking pixels and tracker domains are blocked for senders in your untrusted list.")
            : privacyMode === "relaxed"
            ? t("privacy.relaxedDesc", "Same as Strict. Images load freely; only trackers from untrusted senders are blocked.")
            : t("privacy.offDesc", "No tracking protection. All images and content load without restriction.")}
        </p>
      </div>

      {/* Tracker blocking info */}
      <div style={{
        padding: "12px 16px",
        borderRadius: "6px",
        backgroundColor: "var(--color-bg-hover)",
        marginBottom: "24px",
        fontSize: "13px",
      }}>
        <strong>{t("privacy.trackerBlocking", "How it works")}</strong>
        <p style={{ margin: "4px 0 0", color: "var(--color-text-secondary)", fontSize: "12px" }}>
          {privacyMode === "off"
            ? t("privacy.trackerBlockingOff", "Tracker blocking is disabled in Off mode. All images and content are loaded without restriction.")
            : t("privacy.trackerBlockingDesc", "External images are always loaded. Only known tracking pixels and tracker domains are blocked, and only for senders you've added to your untrusted sender list by tapping the banner at the top of a message.")}
        </p>
      </div>

      {/* Untrusted senders */}
      <div>
        <h3 style={{ fontSize: "14px", fontWeight: 600, marginBottom: "12px" }}>
          {t("privacy.untrustedSenders", "Untrusted Senders")}
        </h3>
        {untrustedSenders.length === 0 ? (
          <p style={{ fontSize: "13px", color: "var(--color-text-secondary)" }}>
            {t("privacy.noUntrustedSenders", "No untrusted senders. By default, all senders are trusted and no tracker blocking is applied. Tap the banner on a message to block a sender's trackers.")}
          </p>
        ) : (
          <div style={{ display: "flex", flexDirection: "column", gap: "4px" }}>
            {untrustedSenders.map((sender) => (
              <div
                key={sender.email}
                style={{
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "space-between",
                  padding: "8px 12px",
                  borderRadius: "6px",
                  border: "1px solid var(--color-border)",
                  fontSize: "13px",
                }}
              >
                <span>{sender.email}</span>
                <button
                  onClick={() => handleRemoveUntrusted(sender.email)}
                  title={t("common.delete", "Delete")}
                  style={{
                    background: "none",
                    border: "none",
                    cursor: "pointer",
                    color: "var(--color-text-secondary)",
                    padding: "4px",
                  }}
                >
                  <Trash2 size={14} />
                </button>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
