import { render, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { ShadowDomEmail } from "@/components/ShadowDomEmail";

describe("ShadowDomEmail", () => {
  it("uses app theme variables instead of hardcoded light text styles", async () => {
    document.documentElement.setAttribute("data-theme", "dark");

    const { container } = render(<ShadowDomEmail html="<p>Hello</p>" />);
    const host = container.firstChild as HTMLDivElement | null;

    await waitFor(() => {
      expect(host?.shadowRoot).not.toBeNull();
    });

    const shadowMarkup = host!.shadowRoot!.innerHTML;
    expect(shadowMarkup).toContain("var(--color-text-primary)");
    expect(shadowMarkup).toContain("var(--color-accent)");
    expect(shadowMarkup).not.toContain("color: #1a1a1a");
  });
});
