import { useCallback, useState } from "react";
import { useTranslation } from "react-i18next";
import { openDevtools, closeDevtools } from "@/lib/api";
import { useUIStore } from "@/stores/ui.store";

export default function DeveloperTab() {
  const { t } = useTranslation();
  const devtoolsAutoOpen = useUIStore((s) => s.devtoolsAutoOpen);
  const setDevtoolsAutoOpen = useUIStore((s) => s.setDevtoolsAutoOpen);
  const [devtoolsOpen, setDevtoolsOpen] = useState(false);

  const toggleDevtools = useCallback(() => {
    if (devtoolsOpen) {
      closeDevtools().catch(() => {});
      setDevtoolsOpen(false);
    } else {
      openDevtools().catch(() => {});
      setDevtoolsOpen(true);
    }
  }, [devtoolsOpen]);

  const toggleAutoOpen = useCallback(() => {
    setDevtoolsAutoOpen(!devtoolsAutoOpen);
    if (!devtoolsAutoOpen) {
      openDevtools().catch(() => {});
      setDevtoolsOpen(true);
    }
  }, [devtoolsAutoOpen, setDevtoolsAutoOpen]);

  return (
    <div>
      <h3 style={{ fontSize: "14px", fontWeight: 600, marginBottom: "16px", marginTop: 0 }}>
        {t("settings.developer", "Developer")}
      </h3>

      <p style={{ fontSize: "12px", color: "var(--color-text-secondary)", marginBottom: "24px", marginTop: 0 }}>
        {t(
          "settings.developerDesc",
          "Tools for debugging the Pebble desktop application."
        )}
      </p>

      <label
        style={{
          display: "flex",
          alignItems: "center",
          gap: "8px",
          cursor: "pointer",
          fontSize: "13px",
          color: "var(--color-text-primary)",
          marginBottom: "16px",
        }}
      >
        <input
          type="checkbox"
          checked={devtoolsOpen}
          onChange={toggleDevtools}
        />
        <span>{t("settings.devtoolsOpen", "DevTools console open")}</span>
      </label>

      <label
        style={{
          display: "flex",
          alignItems: "center",
          gap: "8px",
          cursor: "pointer",
          fontSize: "13px",
          color: "var(--color-text-primary)",
        }}
      >
        <input
          type="checkbox"
          checked={devtoolsAutoOpen}
          onChange={toggleAutoOpen}
        />
        <span>{t("settings.devtoolsAutoOpen", "DevTools console open by default")}</span>
      </label>
    </div>
  );
}
