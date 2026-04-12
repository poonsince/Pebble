import { useRef, useEffect } from "react";

interface ShadowDomEmailProps {
  html: string;
  className?: string;
}

export function ShadowDomEmail({ html, className }: ShadowDomEmailProps) {
  const hostRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!hostRef.current) return;
    const shadow = hostRef.current.shadowRoot
      || hostRef.current.attachShadow({ mode: "open" });

    shadow.innerHTML = `
      <style>
        :host {
          all: initial;
          display: block;
          font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
          font-size: 14px;
          color: var(--color-text-primary);
          background: transparent;
          word-break: break-word;
        }
        img { max-width: 100%; height: auto; }
        a { color: var(--color-accent); }
        pre { white-space: pre-wrap; overflow-x: auto; }
        table { max-width: 100%; border-collapse: collapse; }
        body, div { word-wrap: break-word; overflow-wrap: break-word; }
      </style>
      <div>${html}</div>
    `;
  }, [html]);

  return <div ref={hostRef} className={className} />;
}
