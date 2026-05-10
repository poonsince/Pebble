import { useLayoutEffect, useMemo, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { openMailtoUrl } from "@/app/useMailtoOpen";
import {
  sanitizeHtmlDocumentForIframe,
  wrapHtmlDocumentForIframe,
} from "@/lib/sanitizeHtml";

interface ShadowDomEmailProps {
  html: string;
  className?: string;
}

function isDarkThemeActive(): boolean {
  return document.documentElement.getAttribute("data-theme") === "dark";
}

export function ShadowDomEmail({ html, className }: ShadowDomEmailProps) {
  const iframeRef = useRef<HTMLIFrameElement>(null);
  const cleanupRef = useRef<(() => void) | null>(null);

  const srcDoc = useMemo(() => {
    // eslint-disable-next-line no-console
    console.log(`[ShadowDomEmail] rendering HTML: len=${html.length}, first 200="${html.slice(0, 200)}..."`);
    const safeHtml = sanitizeHtmlDocumentForIframe(html);
    const result = wrapHtmlDocumentForIframe(safeHtml, isDarkThemeActive());
    // eslint-disable-next-line no-console
    console.log(`[ShadowDomEmail] srcDoc produced: len=${result.length}`);
    // eslint-disable-next-line no-console
    console.log(`[ShadowDomEmail] srcDoc FULL:\n${result}`);
    return result;
  }, [html]);

  useLayoutEffect(() => {
    return () => {
      cleanupRef.current?.();
      cleanupRef.current = null;
    };
  }, []);

  const handleLoad = () => {
    cleanupRef.current?.();

    const iframe = iframeRef.current;
    const doc = iframe?.contentDocument;
    if (!iframe || !doc) return;

    const applyHeight = () => {
      const height = Math.max(
        doc.documentElement?.scrollHeight ?? 0,
        doc.body?.scrollHeight ?? 0,
      );
      iframe.style.height = `${height}px`;
    };

    const handleClick = (event: Event) => {
      const target = event.target;
      if (!(target instanceof Element)) return;

      const anchor = target.closest<HTMLAnchorElement>("a[href]");
      const href = anchor?.getAttribute("href")?.trim();
      if (!href) return;

      if (/^mailto:/i.test(href)) {
        event.preventDefault();
        void openMailtoUrl(href);
        return;
      }

      if (/^https?:\/\//i.test(href)) {
        event.preventDefault();
        void invoke("open_external_url", { url: href })
          .catch((err) => console.warn("Failed to open email body link", err));
      }
    };

    applyHeight();
    requestAnimationFrame(applyHeight);
    window.setTimeout(applyHeight, 60);
    window.setTimeout(applyHeight, 180);

    doc.addEventListener("click", handleClick);

    const observer = typeof ResizeObserver === "undefined"
      ? null
      : new ResizeObserver(() => applyHeight());

    if (observer && doc.documentElement) {
      observer.observe(doc.documentElement);
    }

    cleanupRef.current = () => {
      doc.removeEventListener("click", handleClick);
      observer?.disconnect();
    };
  };

  return (
    <iframe
      ref={iframeRef}
      className={className}
      sandbox="allow-same-origin"
      srcDoc={srcDoc}
      onLoad={handleLoad}
      title="Email content"
      scrolling="no"
      style={{
        width: "100%",
        border: "none",
        display: "block",
        background: "transparent",
      }}
    />
  );
}
