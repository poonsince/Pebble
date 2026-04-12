import DOMPurify from "dompurify";

/** Sanitize HTML to prevent XSS while preserving email formatting. */
export function sanitizeHtml(html: string): string {
  return DOMPurify.sanitize(html, {
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
      "dir", "id", "lang", "colspan", "rowspan", "border", "cellpadding",
      "cellspacing", "align", "valign", "bgcolor", "color", "face", "size",
    ],
    ALLOW_DATA_ATTR: false,
  });
}
