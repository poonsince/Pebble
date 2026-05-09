import DOMPurify from "dompurify";

const SAFE_STYLE_PROPERTIES = new Set([
  "background",
  "background-color",
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
  "color",
  "display",
  "font",
  "font-family",
  "font-size",
  "font-style",
  "font-weight",
  "height",
  "line-height",
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
  "padding",
  "padding-bottom",
  "padding-left",
  "padding-right",
  "padding-top",
  "text-align",
  "text-decoration",
  "vertical-align",
  "visibility",
  "white-space",
  "width",
]);

function isSafeBackgroundShorthandValue(value: string): boolean {
  const normalized = value
    .trim()
    .replace(/\s*!important\s*$/i, "")
    .trim()
    .toLowerCase();

  if (!normalized) return false;
  if (
    /(url\s*\(|image-set\s*\(|-webkit-image-set\s*\(|cross-fade\s*\(|element\s*\(|paint\s*\(|expression\s*\(|javascript:|vbscript:|data:|@import|\\)/i.test(
      normalized,
    )
  ) {
    return false;
  }
  if (["none", "transparent", "currentcolor"].includes(normalized)) return true;
  if (/^#[0-9a-f]{3,8}$/i.test(normalized)) return true;
  if (/^(rgb|rgba|hsl|hsla)\([\d\s.,%/+-]+\)$/i.test(normalized)) return true;
  return /^[a-z]+$/.test(normalized);
}

function filterStyleAttribute(style: string): string {
  return style
    .split(";")
    .map((part) => part.trim())
    .filter(Boolean)
    .filter((part) => {
      const [rawName, ...rawValue] = part.split(":");
      const name = rawName.trim().toLowerCase();
      const value = rawValue.join(":").trim().toLowerCase();
      if (!SAFE_STYLE_PROPERTIES.has(name) || !value) return false;
      if (name === "background") return isSafeBackgroundShorthandValue(value);
      if (value.includes("\\")) return false;
      return !/(url\s*\(|expression\s*\(|javascript:|vbscript:|data:|@import)/i.test(value);
    })
    .join("; ");
}

function removeUnsafeCss(content: string): string {
  return content
    .replace(/\/\*[\s\S]*?\*\//g, "")
    .replace(/@import[^;]+;/gi, "")
    .replace(/[^{}]+{[^{}]*}/g, (rule) => {
      const openBrace = rule.indexOf("{");
      const closeBrace = rule.lastIndexOf("}");
      if (openBrace === -1 || closeBrace === -1) return "";
      const selector = rule.slice(0, openBrace).trim();
      const body = rule.slice(openBrace + 1, closeBrace).trim();
      const filtered = filterStyleAttribute(body).replace(/;\s+/g, ";");
      if (!selector || !filtered) return "";
      return `${selector}{${filtered}}`;
    });
}

function filterInlineStyles(html: string): string {
  const template = document.createElement("template");
  template.innerHTML = html;
  template.content.querySelectorAll<HTMLElement>("[style]").forEach((element) => {
    const filtered = filterStyleAttribute(element.getAttribute("style") ?? "");
    if (filtered) {
      element.setAttribute("style", filtered);
    } else {
      element.removeAttribute("style");
    }
  });
  return template.innerHTML;
}

function filterStyleTags(html: string): string {
  const template = document.createElement("template");
  template.innerHTML = html;
  template.content.querySelectorAll("style").forEach((styleElement) => {
    const filtered = removeUnsafeCss(styleElement.textContent ?? "");
    if (filtered.trim()) {
      styleElement.textContent = filtered;
    } else {
      styleElement.remove();
    }
  });
  return template.innerHTML;
}

function removeEventHandlerAttributes(html: string): string {
  const template = document.createElement("template");
  template.innerHTML = html;
  template.content.querySelectorAll("*").forEach((element) => {
    for (const attribute of Array.from(element.attributes)) {
      if (/^on/i.test(attribute.name)) {
        element.removeAttribute(attribute.name);
      }
    }
  });
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
  const source = looksLikeFullDocument(html) ? extractBodyContent(html) : html;
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
  return normalizeLinkAttributes(filterInlineStyles(sanitized));
}

export function sanitizeHtmlDocumentForIframe(html: string): string {
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
  const filtered = normalizeLinkAttributes(
    removeEventHandlerAttributes(filterStyleTags(filterInlineStyles(sanitized))),
  );

  if (!looksLikeFullDocument(html)) {
    return filtered;
  }

  const sourceDocument = new DOMParser().parseFromString(html, "text/html");
  const filteredDocument = new DOMParser().parseFromString(filtered, "text/html");
  const outputDocument = new DOMParser().parseFromString(
    "<!doctype html><html><head></head><body></body></html>",
    "text/html",
  );

  sourceDocument.head.querySelectorAll("meta[charset], meta[name], title, style").forEach((node) => {
    const clone = node.cloneNode(true);
    if (clone instanceof Element && clone.tagName.toLowerCase() === "style") {
      const css = removeUnsafeCss(clone.textContent ?? "");
      if (!css.trim()) return;
      clone.textContent = css;
    }
    outputDocument.head.append(clone);
  });
  outputDocument.body.innerHTML = filteredDocument.body.innerHTML;
  return outputDocument.documentElement.outerHTML;
}

export function wrapHtmlDocumentForIframe(html: string, useDarkFallback: boolean): string {
  const supportStyles = buildIframeSupportStyles(useDarkFallback);
  if (looksLikeFullDocument(html)) {
    const document_ = new DOMParser().parseFromString(html, "text/html");
    ensureIframeDocumentHead(document_, supportStyles);
    return `<!doctype html>${document_.documentElement.outerHTML}`;
  }

  const escapedHtml = sanitizeHtmlDocumentForIframe(html);
  const document_ = new DOMParser().parseFromString("<!doctype html><html><head></head><body></body></html>", "text/html");
  document_.body.innerHTML = escapedHtml;
  ensureIframeDocumentHead(document_, supportStyles);
  return `<!doctype html>${document_.documentElement.outerHTML}`;
}
