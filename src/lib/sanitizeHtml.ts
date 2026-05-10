import DOMPurify from "dompurify";

// NOTE: Keep this in sync with the Rust backend's SAFE_PROPERTIES
// in crates/pebble-privacy/src/sanitizer.rs
const SAFE_STYLE_PROPERTIES = new Set([
  "background",
  "background-color",
  "background-image",
  "background-repeat",
  "background-size",
  "background-position",
  "background-position-x",
  "background-position-y",
  "border",
  "border-bottom",
  "border-collapse",
  "border-color",
  "border-left",
  "border-right",
  "border-spacing",
  "border-style",
  "border-top",
  "border-width",
  "box-sizing",
  "clear",
  "color",
  "direction",
  "display",
  "float",
  "font",
  "font-family",
  "font-size",
  "font-style",
  "font-variant",
  "font-weight",
  "height",
  "letter-spacing",
  "line-height",
  "list-style",
  "list-style-position",
  "list-style-type",
  "margin",
  "margin-bottom",
  "margin-left",
  "margin-right",
  "margin-top",
  "max-height",
  "max-width",
  "min-height",
  "min-width",
  "opacity",
  "overflow",
  "overflow-x",
  "overflow-y",
  "overflow-wrap",
  "padding",
  "padding-bottom",
  "padding-left",
  "padding-right",
  "padding-top",
  "position",
  "outline",
  "border-radius",
  "content",
  "table-layout",
  "text-align",
  "text-decoration",
  "text-indent",
  "text-transform",
  "vertical-align",
  "visibility",
  "white-space",
  "width",
  "word-break",
  "word-spacing",
  "word-wrap",
]);

const DEBUG_SANITIZE = true;

function debugLog(tag: string, ...args: unknown[]) {
  if (DEBUG_SANITIZE) {
    // eslint-disable-next-line no-console
    console.log(`[sanitizeHtml:${tag}]`, ...args);
  }
}

function isSafeBackgroundShorthandValue(value: string): boolean {
  const normalized = value
    .trim()
    .replace(/\s*!important\s*$/i, "")
    .trim()
    .toLowerCase();

  if (!normalized) {
    debugLog("background-check", "empty value -> reject");
    return false;
  }
  if (
    /(expression\s*\(|javascript:|vbscript:|data:|@import|\\)/i.test(
      normalized,
    )
  ) {
    debugLog("background-check", `dangerous pattern in value: "${normalized}" -> reject`);
    return false;
  }

  // url() in background shorthand is now allowed in all privacy modes
  if (/url\s*\(/i.test(normalized)) {
    debugLog("background-check", `url() allowed: "${normalized.slice(0, 80)}"`);
    return true;
  }

  if (["none", "transparent", "currentcolor"].includes(normalized)) return true;
  if (/^#[0-9a-f]{3,8}$/i.test(normalized)) return true;
  if (/^(rgb|rgba|hsl|hsla)\([\d\s.,%/+-]+\)$/i.test(normalized)) return true;
  return /^[a-z]+$/.test(normalized);
}

function filterStyleAttribute(style: string): string {
  const parts = style
    .split(";")
    .map((part) => part.trim())
    .filter(Boolean);
  const beforeCount = parts.length;
  const allowed = parts.filter((part) => {
    const [rawName, ...rawValue] = part.split(":");
    const name = rawName.trim().toLowerCase();
    const value = rawValue.join(":").trim().toLowerCase();
    if (!SAFE_STYLE_PROPERTIES.has(name) || !value) return false;
    if (name === "background") return isSafeBackgroundShorthandValue(value);
    if (value.includes("\\")) return false;
    return !/(expression\s*\(|javascript:|vbscript:|data:|@import)/i.test(value);
  });
  const result = allowed.join("; ");
  if (allowed.length !== beforeCount) {
    debugLog("filterStyleAttribute", `removed ${beforeCount - allowed.length}/${beforeCount} props; input: "${style.slice(0, 100)}..."`);
  }
  return result;
}

function removeUnsafeCss(content: string): string {
  const beforeLen = content.length;
  // Remove CSS comments
  const noComments = content.replace(/\/\*[\s\S]*?\*\//g, "");
  if (noComments.length !== beforeLen) {
    debugLog("removeUnsafeCss-comments", `removed ${beforeLen - noComments.length} chars of comments`);
  }

  // Remove @import rules
  const noImport = noComments.replace(/@import[^;]+;/gi, "");
  if (noImport.length !== noComments.length) {
    debugLog("removeUnsafeCss-import", `removed @import rules (${noComments.length - noImport.length} chars)`);
  }

  const result = noImport.replace(/[^{}]+{[^{}]*}/g, (rule) => {
    const openBrace = rule.indexOf("{");
    const closeBrace = rule.lastIndexOf("}");
    if (openBrace === -1 || closeBrace === -1) {
      debugLog("removeUnsafeCss-rule", "malformed rule (no braces) -> dropped");
      return "";
    }
    const selector = rule.slice(0, openBrace).trim();
    const body = rule.slice(openBrace + 1, closeBrace).trim();
    if (!selector || !body) {
      debugLog("removeUnsafeCss-rule", `empty selector or body -> dropped`);
      return "";
    }
    const filtered = filterStyleAttribute(body).replace(/;\s+/g, ";");
    if (!filtered) {
      debugLog("removeUnsafeCss-rule", `all props filtered out for selector "${selector.slice(0, 80)}..." -> rule dropped`);
      return "";
    }
    if (filtered !== body) {
      debugLog("removeUnsafeCss-rule", `selector "${selector.slice(0, 80)}..." props filtered: ${body.slice(0, 100)} => ${filtered.slice(0, 100)}`);
    }
    return `${selector}{${filtered}}`;
  });

  debugLog("removeUnsafeCss", `length ${beforeLen} -> ${result.length}`);
  return result;
}

function filterInlineStyles(html: string): string {
  const template = document.createElement("template");
  template.innerHTML = html;
  const els = template.content.querySelectorAll<HTMLElement>("[style]");
  if (els.length > 0) {
    debugLog("filterInlineStyles", `processing ${els.length} elements with inline styles`);
  }
  els.forEach((element) => {
    const original = element.getAttribute("style") ?? "";
    const filtered = filterStyleAttribute(original);
    if (filtered) {
      if (filtered !== original) {
        debugLog("filterInlineStyles", `element <${element.tagName.toLowerCase()}> style changed`);
      }
      element.setAttribute("style", filtered);
    } else {
      debugLog("filterInlineStyles", `element <${element.tagName.toLowerCase()}> style fully removed: "${original.slice(0, 80)}"`);
      element.removeAttribute("style");
    }
  });
  return template.innerHTML;
}

function filterStyleTags(html: string): string {
  const template = document.createElement("template");
  template.innerHTML = html;
  const styleTags = template.content.querySelectorAll("style");
  let kept = 0;
  let removed = 0;
  styleTags.forEach((styleElement) => {
    const originalLen = (styleElement.textContent ?? "").length;
    const filtered = removeUnsafeCss(styleElement.textContent ?? "");
    if (filtered.trim()) {
      styleElement.textContent = filtered;
      if (filtered.length < originalLen) {
        debugLog("filterStyleTags", `style tag trimmed: ${originalLen} -> ${filtered.length} chars`);
      }
      kept++;
    } else {
      styleElement.remove();
      debugLog("filterStyleTags", `style tag removed (all CSS unsafe or empty)`);
      removed++;
    }
  });
  if (kept + removed > 0) {
    debugLog("filterStyleTags", `${kept} kept, ${removed} removed`);
  }
  return template.innerHTML;
}

function removeEventHandlerAttributes(html: string): string {
  const template = document.createElement("template");
  template.innerHTML = html;
  let count = 0;
  template.content.querySelectorAll("*").forEach((element) => {
    for (const attribute of Array.from(element.attributes)) {
      if (/^on/i.test(attribute.name)) {
        element.removeAttribute(attribute.name);
        count++;
      }
    }
  });
  if (count > 0) {
    debugLog("removeEventHandlerAttributes", `removed ${count} event handler attributes`);
  }
  return template.innerHTML;
}

function normalizeLinkAttributes(html: string): string {
  const template = document.createElement("template");
  template.innerHTML = html;
  template.content.querySelectorAll<HTMLAnchorElement>("a[href]").forEach((anchor) => {
    const href = anchor.getAttribute("href")?.trim() ?? "";
    if (/^(https?:|mailto:)/i.test(href)) {
      anchor.setAttribute("target", "_blank");
      anchor.setAttribute("rel", "noopener noreferrer");
    } else {
      anchor.removeAttribute("target");
      anchor.removeAttribute("rel");
    }
  });
  return template.innerHTML;
}

function looksLikeFullDocument(html: string): boolean {
  return /<(?:!doctype|html|head|body)\b/i.test(html);
}

function extractBodyContent(html: string): string {
  const document_ = new DOMParser().parseFromString(html, "text/html");
  return document_.body.innerHTML;
}

function ensureIframeDocumentHead(document_: Document, supportStyles: string): void {
  if (!document_.head) {
    const head = document_.createElement("head");
    document_.documentElement.insertBefore(head, document_.body ?? null);
  }

  if (!document_.head.querySelector('meta[charset]')) {
    const meta = document_.createElement("meta");
    meta.setAttribute("charset", "utf-8");
    document_.head.prepend(meta);
  }

  // Send full Referer for subresource requests (images, etc.) so CDNs
  // with hotlink protection (e.g. NetEase nosdn.127.net) accept them.
  if (!document_.head.querySelector('meta[name="referrer"]')) {
    const referrer = document_.createElement("meta");
    referrer.setAttribute("name", "referrer");
    referrer.setAttribute("content", "unsafe-url");
    document_.head.prepend(referrer);
  }

  if (!document_.head.querySelector('meta[name="viewport"]')) {
    const meta = document_.createElement("meta");
    meta.setAttribute("name", "viewport");
    meta.setAttribute("content", "width=device-width, initial-scale=1");
    document_.head.append(meta);
  }

  const style = document_.createElement("style");
  style.textContent = supportStyles;
  document_.head.append(style);
}

function buildIframeSupportStyles(useDarkFallback: boolean): string {
  const rules = [
    "img { max-width: 100% !important; height: auto !important; }",
    "table { max-width: 100% !important; }",
    "pre { white-space: pre-wrap; overflow-x: auto; }",
  ];

  if (useDarkFallback) {
    rules.push("body { background: #fff; color: #202124; color-scheme: light; }");
  }

  return rules.join("\n");
}

/** Sanitize HTML to prevent XSS while preserving email formatting. */
export function sanitizeHtml(html: string): string {
  const isFullDoc = looksLikeFullDocument(html);
  debugLog("sanitizeHtml", `input len=${html.length}, isFullDoc=${isFullDoc}`);
  if (isFullDoc) {
    debugLog("sanitizeHtml", "extracting body content from full document");
  }
  const source = isFullDoc ? extractBodyContent(html) : html;
  const sanitized = DOMPurify.sanitize(source, {
    ALLOWED_TAGS: [
      "a", "abbr", "address", "article", "b", "bdi", "bdo", "blockquote",
      "br", "caption", "center", "cite", "code", "col", "colgroup", "dd", "del",
      "details", "dfn", "div", "dl", "dt", "em", "figcaption", "figure",
      "font", "footer", "h1", "h2", "h3", "h4", "h5", "h6", "header", "hr", "i",
      "img", "ins", "kbd", "li", "main", "mark", "nav", "ol", "p", "pre",
      "q", "rp", "rt", "ruby", "s", "samp", "section", "small", "span",
      "strong", "sub", "summary", "sup", "table", "tbody", "td", "tfoot",
      "th", "thead", "time", "tr", "u", "ul", "var", "wbr",
    ],
    ALLOWED_ATTR: [
      "href", "src", "alt", "title", "width", "height", "class",
      "target", "rel",
      "dir", "id", "lang", "colspan", "rowspan", "border", "cellpadding",
      "cellspacing", "align", "valign", "bgcolor", "color", "face", "size",
      "style", "data-cid", "data-src",
    ],
    ALLOW_DATA_ATTR: false,
  });
  debugLog("sanitizeHtml-1", `DOMPurify done: ${source.length} -> ${sanitized.length}`);
  const result = normalizeLinkAttributes(filterInlineStyles(sanitized));
  debugLog("sanitizeHtml-end", `final len=${result.length}`);
  return result;
}

export function sanitizeHtmlDocumentForIframe(html: string): string {
  const isFullDoc = looksLikeFullDocument(html);
  debugLog("sanitizeHtmlDocumentForIframe", `input len=${html.length}, isFullDoc=${isFullDoc}`);

  const sanitized = DOMPurify.sanitize(html, {
    ALLOWED_TAGS: [
      "html", "head", "body", "meta", "title", "style",
      "a", "abbr", "address", "article", "b", "bdi", "bdo", "blockquote",
      "br", "caption", "center", "cite", "code", "col", "colgroup", "dd", "del",
      "details", "dfn", "div", "dl", "dt", "em", "figcaption", "figure",
      "font", "footer", "h1", "h2", "h3", "h4", "h5", "h6", "header", "hr", "i",
      "img", "ins", "kbd", "li", "main", "mark", "nav", "ol", "p", "pre",
      "q", "rp", "rt", "ruby", "s", "samp", "section", "small", "span",
      "strong", "sub", "summary", "sup", "table", "tbody", "td", "tfoot",
      "th", "thead", "time", "tr", "u", "ul", "var", "wbr",
    ],
    ALLOWED_ATTR: [
      "href", "src", "alt", "title", "width", "height", "class",
      "target", "rel",
      "dir", "id", "lang", "colspan", "rowspan", "border", "cellpadding",
      "cellspacing", "align", "valign", "bgcolor", "color", "face", "size",
      "style", "content", "name", "charset", "data-cid", "data-src",
    ],
    FORBID_TAGS: ["script"],
    ALLOW_DATA_ATTR: false,
  });
  debugLog("sanitizeHtmlDocumentForIframe-1", `DOMPurify done: ${html.length} -> ${sanitized.length}`);

  const withEventRemoved = removeEventHandlerAttributes(sanitized);
  const withStyleFiltered = filterStyleTags(withEventRemoved);
  const withInlineFiltered = filterInlineStyles(withStyleFiltered);
  const filtered = normalizeLinkAttributes(withInlineFiltered);
  debugLog("sanitizeHtmlDocumentForIframe-2", `post-processing done: ${filtered.length} chars`);

  if (!isFullDoc) {
    debugLog("sanitizeHtmlDocumentForIframe", "not a full document, returning fragment");
    return filtered;
  }

  // Rebuild full document preserving head metadata (style, meta, title, charset)
  const sourceDocument = new DOMParser().parseFromString(html, "text/html");
  const filteredDocument = new DOMParser().parseFromString(filtered, "text/html");
  const outputDocument = new DOMParser().parseFromString(
    "<!doctype html><html><head></head><body></body></html>",
    "text/html",
  );

  let stylePreserved = 0;
  let styleStripped = 0;
  sourceDocument.head.querySelectorAll("meta[charset], meta[name], title, style").forEach((node) => {
    const clone = node.cloneNode(true);
    if (clone instanceof Element && clone.tagName.toLowerCase() === "style") {
      const css = removeUnsafeCss(clone.textContent ?? "");
      if (!css.trim()) {
        styleStripped++;
        return;
      }
      clone.textContent = css;
      stylePreserved++;
    }
    outputDocument.head.append(clone);
  });

  if (stylePreserved + styleStripped > 0) {
    debugLog("sanitizeHtmlDocumentForIframe-style-preserve", `${stylePreserved} kept, ${styleStripped} stripped from head`);
  }

  outputDocument.body.innerHTML = filteredDocument.body.innerHTML;
  debugLog("sanitizeHtmlDocumentForIframe-end", `final len=${outputDocument.documentElement.outerHTML.length}`);
  return outputDocument.documentElement.outerHTML;
}

export function wrapHtmlDocumentForIframe(html: string, useDarkFallback: boolean): string {
  debugLog("wrapHtmlDocumentForIframe", `input len=${html.length}, useDarkFallback=${useDarkFallback}`);
  const supportStyles = buildIframeSupportStyles(useDarkFallback);
  if (looksLikeFullDocument(html)) {
    debugLog("wrapHtmlDocumentForIframe", "looks like full document, embedding directly");
    const document_ = new DOMParser().parseFromString(html, "text/html");
    ensureIframeDocumentHead(document_, supportStyles);
    const result = `<!doctype html>${document_.documentElement.outerHTML}`;
    debugLog("wrapHtmlDocumentForIframe-end", `output len=${result.length}`);
    return result;
  }

  debugLog("wrapHtmlDocumentForIframe", "not a full document, sanitizing then wrapping");
  const escapedHtml = sanitizeHtmlDocumentForIframe(html);
  debugLog("wrapHtmlDocumentForIframe", `sanitized len=${escapedHtml.length}`);
  const document_ = new DOMParser().parseFromString("<!doctype html><html><head></head><body></body></html>", "text/html");
  document_.body.innerHTML = escapedHtml;
  ensureIframeDocumentHead(document_, supportStyles);
  const result = `<!doctype html>${document_.documentElement.outerHTML}`;
  debugLog("wrapHtmlDocumentForIframe-end", `output len=${result.length}`);
  return result;
}
