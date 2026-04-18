import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

describe("focus-visible CSS", () => {
  it("does not suppress the Tiptap editor focus outline", () => {
    const css = readFileSync(join(process.cwd(), "src", "styles", "index.css"), "utf8");

    expect(css).not.toMatch(/\.tiptap\s*\{[^}]*outline\s*:\s*none/i);
    expect(css).not.toMatch(/\.tiptap:focus\s*\{[^}]*outline\s*:\s*none/i);
  });
});
