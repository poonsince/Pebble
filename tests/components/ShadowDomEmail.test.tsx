import { fireEvent, render, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { ShadowDomEmail } from "@/components/ShadowDomEmail";

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
  openMailtoUrl: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: mocks.invoke,
}));

vi.mock("@/app/useMailtoOpen", () => ({
  openMailtoUrl: mocks.openMailtoUrl,
}));

describe("ShadowDomEmail", () => {
  beforeEach(() => {
    document.documentElement.removeAttribute("data-theme");
    mocks.invoke.mockReset();
    mocks.openMailtoUrl.mockReset();
    mocks.invoke.mockResolvedValue(undefined);
    mocks.openMailtoUrl.mockResolvedValue(true);
  });

  it("renders sanitized email html into iframe srcdoc", () => {
    const { container } = render(
      <ShadowDomEmail html="<html><head><style>.card{max-width:600px}</style></head><body><div class='card'>Hello</div></body></html>" />,
    );

    const iframe = container.querySelector("iframe");
    expect(iframe).not.toBeNull();
    // sandbox intentionally removed (causes CDN 405 errors)
    expect(iframe?.getAttribute("sandbox")).toBeNull();
    expect(iframe?.getAttribute("srcdoc")).toContain(".card{max-width:600px}");
    expect(iframe?.getAttribute("srcdoc")).toContain("Hello");
  });

  it("adds a light reading canvas fallback in dark theme", () => {
    document.documentElement.setAttribute("data-theme", "dark");

    const { container } = render(<ShadowDomEmail html="<p>Hello</p>" />);

    const iframe = container.querySelector("iframe");
    expect(iframe?.getAttribute("srcdoc")).toContain("background: #fff; color: #202124; color-scheme: light;");
  });

  it("opens http and https links through the external URL command", async () => {
    const { container } = render(
      <ShadowDomEmail html={'<a href="http://pebble.byebug.cn/">Pebble</a>'} />,
    );

    const iframe = container.querySelector("iframe") as HTMLIFrameElement;
    const anchor = document.createElement("a");
    anchor.setAttribute("href", "http://pebble.byebug.cn/");
    const iframeDocument = {
      addEventListener: vi.fn((event: string, handler: EventListener) => {
        if (event === "click") {
          handler({
            target: anchor,
            preventDefault: vi.fn(),
          } as unknown as Event);
        }
      }),
      removeEventListener: vi.fn(),
      documentElement: { scrollHeight: 480 },
      body: { scrollHeight: 420 },
    } as unknown as Document;

    Object.defineProperty(iframe, "contentDocument", {
      configurable: true,
      value: iframeDocument,
    });

    fireEvent.load(iframe);

    await waitFor(() => {
      expect(mocks.invoke).toHaveBeenCalledWith("open_external_url", {
        url: "http://pebble.byebug.cn/",
      });
    });
  });

  it("opens mailto links through the compose mailto handler", async () => {
    const { container } = render(
      <ShadowDomEmail html={'<a href="mailto:qingj1314@163.com">qingj1314@163.com</a>'} />,
    );

    const iframe = container.querySelector("iframe") as HTMLIFrameElement;
    const anchor = document.createElement("a");
    anchor.setAttribute("href", "mailto:qingj1314@163.com");
    const iframeDocument = {
      addEventListener: vi.fn((event: string, handler: EventListener) => {
        if (event === "click") {
          handler({
            target: anchor,
            preventDefault: vi.fn(),
          } as unknown as Event);
        }
      }),
      removeEventListener: vi.fn(),
      documentElement: { scrollHeight: 360 },
      body: { scrollHeight: 320 },
    } as unknown as Document;

    Object.defineProperty(iframe, "contentDocument", {
      configurable: true,
      value: iframeDocument,
    });

    fireEvent.load(iframe);

    await waitFor(() => {
      expect(mocks.openMailtoUrl).toHaveBeenCalledWith("mailto:qingj1314@163.com");
    });
    expect(mocks.invoke).not.toHaveBeenCalled();
  });

  it("sizes the iframe to the loaded document height", async () => {
    const { container } = render(<ShadowDomEmail html="<p>Hello</p>" />);
    const iframe = container.querySelector("iframe") as HTMLIFrameElement;
    const iframeDocument = {
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      documentElement: { scrollHeight: 640 },
      body: { scrollHeight: 512 },
    } as unknown as Document;

    Object.defineProperty(iframe, "contentDocument", {
      configurable: true,
      value: iframeDocument,
    });

    fireEvent.load(iframe);

    await waitFor(() => {
      expect(iframe.style.height).toBe("640px");
    });
  });
});
