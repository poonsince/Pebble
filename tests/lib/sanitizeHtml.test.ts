import { describe, expect, it } from "vitest";
import {
  sanitizeHtml,
  sanitizeHtmlDocumentForIframe,
  wrapHtmlDocumentForIframe,
} from "../../src/lib/sanitizeHtml";

describe("sanitizeHtml", () => {
  it("preserves safe inline email styles", () => {
    const sanitized = sanitizeHtml(
      '<p style="color: red; text-align: center; margin: 8px">Hello</p>',
    );

    expect(sanitized).toContain("style=");
    expect(sanitized).toContain("color:");
    expect(sanitized).toContain("text-align:");
  });

  it("removes unsafe inline style content", () => {
    const sanitized = sanitizeHtml(
      '<p style="background-image: url(javascript:alert(1)); color: blue">Hello</p>',
    );

    expect(sanitized).not.toContain("javascript:");
    expect(sanitized).toContain("color:");
  });

  it("preserves safe background shorthand used by email buttons", () => {
    const sanitized = sanitizeHtml(
      '<a style="background: #f38020; color: #ffffff; border: 1px solid #f38020">Open dashboard</a>',
    );

    expect(sanitized).toContain("background:");
    expect(sanitized).toContain("#f38020");
    expect(sanitized).toContain("color:");
  });

  it("removes unsafe background shorthand urls", () => {
    const sanitized = sanitizeHtml(
      '<p style="background: url(https://evil.example/track); color: blue">Hello</p>',
    );

    expect(sanitized).not.toContain("evil.example");
    expect(sanitized).toContain("color:");
  });

  it("keeps zero-height email spacer image constraints", () => {
    const sanitized = sanitizeHtml(
      '<img src="https://example.com/spacer.png" width="600" height="1" style="display:block;max-height:0px;min-height:0px;min-width:600px;width:600px">',
    );

    expect(sanitized).toContain("max-height:0px");
    expect(sanitized).toContain("min-height:0px");
    expect(sanitized).toContain("min-width:600px");
    expect(sanitized).toContain('height="1"');
  });

  it("preserves hidden preheader clipping styles", () => {
    const sanitized = sanitizeHtml(
      '<div style="max-width:0px;max-height:0px;overflow:hidden;visibility:hidden;opacity:0">马凯，为您推荐 2 条新动态</div>',
    );

    expect(sanitized).toContain("max-width:0px");
    expect(sanitized).toContain("max-height:0px");
    expect(sanitized).toContain("overflow:hidden");
    expect(sanitized).toContain("visibility:hidden");
    expect(sanitized).toContain("opacity:0");
  });

  it("uses only body content from full html documents", () => {
    const sanitized = sanitizeHtml(
      "<html><head><title>Leaked subject</title><style>p{color:red}</style></head><body><p>Visible body</p></body></html>",
    );

    expect(sanitized).toContain("Visible body");
    expect(sanitized).not.toContain("Leaked subject");
    expect(sanitized).not.toContain("p{color:red}");
  });

  it("removes inline styles with escaped url tokens", () => {
    const sanitized = sanitizeHtml(
      `<p style="color: u\\72l('https://evil.example/track')">hello</p>`,
    );

    expect(sanitized).not.toContain("evil.example");
    expect(sanitized).not.toContain("u\\72l");
  });

  it("preserves safe link attributes for email body links", () => {
    const sanitized = sanitizeHtml(
      '<a href="https://example.com" target="_blank" rel="noopener noreferrer">https://example.com</a>',
    );

    expect(sanitized).toContain('href="https://example.com"');
    expect(sanitized).toContain('target="_blank"');
    expect(sanitized).toContain('rel="noopener noreferrer"');
  });

  it("normalizes existing email links to safe external navigation attributes", () => {
    const sanitized = sanitizeHtml(
      '<a href="mailto:support@example.com" target="_top" rel="opener">support@example.com</a>',
    );

    expect(sanitized).toContain('href="mailto:support@example.com"');
    expect(sanitized).toContain('target="_blank"');
    expect(sanitized).toContain('rel="noopener noreferrer"');
    expect(sanitized).not.toContain("_top");
    expect(sanitized).not.toContain('rel="opener"');
  });
});

describe("sanitizeHtmlDocumentForIframe", () => {
  it("preserves safe head styles for full html email documents", () => {
    const sanitized = sanitizeHtmlDocumentForIframe(
      '<html><head><style>.card{max-width:600px;margin:0 auto;background:#fff}</style><title>Subject</title></head><body><div class="card">Visible body</div></body></html>',
    );

    expect(sanitized).toContain("<html");
    expect(sanitized).toContain("<head>");
    expect(sanitized).toContain(".card{max-width:600px;margin:0 auto;background:#fff}");
    expect(sanitized).toContain("Visible body");
    expect(sanitized).toContain("<title>Subject</title>");
  });

  it("removes scripts, event handlers, and unsafe css from iframe html", () => {
    const sanitized = sanitizeHtmlDocumentForIframe(
      '<html><head><style>body{background:url(https://evil.example/track)}</style><script>alert(1)</script></head><body><a href="javascript:alert(1)" onclick="alert(1)">Open</a><p>Body</p></body></html>',
    );

    expect(sanitized).toContain("Body");
    expect(sanitized).not.toContain("<script");
    expect(sanitized).not.toContain("onclick=");
    expect(sanitized).not.toContain("javascript:");
    expect(sanitized).not.toContain("evil.example");
  });
});

describe("wrapHtmlDocumentForIframe", () => {
  it("wraps body fragments in a full html document with support styles", () => {
    const wrapped = wrapHtmlDocumentForIframe("<p>Hello</p>", false);

    expect(wrapped).toContain("<!doctype html>");
    expect(wrapped).toContain('<meta charset="utf-8">');
    expect(wrapped).toContain('name="viewport"');
    expect(wrapped).toContain("<p>Hello</p>");
    expect(wrapped).toContain("img { max-width: 100% !important; height: auto !important; }");
  });
});
