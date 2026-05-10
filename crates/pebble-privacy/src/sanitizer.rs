use std::collections::HashSet;

use ammonia::Builder;
use pebble_core::{PrivacyMode, RenderedHtml, TrackerInfo};
use tracing::{debug, info, trace};

/// Safely truncate a string to at most `max_chars` UTF-8 characters,
/// returning a `&str` that is guaranteed to end on a char boundary.
fn char_safe_truncate<'a>(s: &'a str, max_chars: usize) -> &'a str {
    let byte_end = s
        .char_indices()
        .nth(max_chars)
        .map(|(i, _)| i)
        .unwrap_or(s.len());
    &s[..byte_end]
}

use crate::tracker::{is_known_tracker, is_tracking_pixel};

pub struct PrivacyGuard;

impl PrivacyGuard {
    pub fn new() -> Self {
        Self
    }

    pub fn render_message_html(
        &self,
        raw_html: &str,
        body_text: &str,
        mode: &PrivacyMode,
    ) -> RenderedHtml {
        let source_html = if raw_html.trim().is_empty() && !body_text.is_empty() {
            debug!(
                body_len = body_text.len(),
                "render_message_html: no HTML, wrapping plain text"
            );
            format!(
                r#"<pre class="pebble-plain-text-email">{}</pre>"#,
                html_escape(body_text)
            )
        } else {
            raw_html.to_string()
        };

        debug!(
            raw_len = source_html.len(),
            body_len = body_text.len(),
            ?mode,
            "render_message_html: starting HTML rendering"
        );
        let mut rendered = self.render_safe_html(&source_html, mode);
        rendered.html = linkify_html_text_nodes(&rendered.html);
        debug!(
            rendered_len = rendered.html.len(),
            trackers = rendered.trackers_blocked.len(),
            images_blocked = rendered.images_blocked,
            "render_message_html: complete"
        );
        rendered
    }

    pub fn render_safe_html(&self, raw_html: &str, mode: &PrivacyMode) -> RenderedHtml {
        info!(
            input_len = raw_html.len(),
            ?mode,
            "render_safe_html: starting"
        );

        let mut trackers_blocked: Vec<TrackerInfo> = Vec::new();
        let mut images_blocked: u32 = 0;

        // Extract and clean <style> blocks BEFORE body extraction and
        // ammonia sanitization. We preserve them separately because ammonia
        // strips <style> tags (it treats them as `clean_content_tags`).
        let (html_without_styles, cleaned_styles) = extract_and_clean_styles(raw_html);
        trace!(
            after_extract_len = html_without_styles.len(),
            "render_safe_html: extract_and_clean_styles done"
        );

        if cleaned_styles.is_empty() {
            info!("render_safe_html: no <style> blocks found in input");
            // Log first 500 chars of raw HTML to help debug missing styles
            let preview = char_safe_truncate(raw_html, 500);
            debug!(raw_preview = %preview, "render_safe_html: raw HTML preview (first 500 chars)");
        } else {
            info!(
                count = cleaned_styles.len(),
                total_css_len = cleaned_styles.iter().map(|s| s.len()).sum::<usize>(),
                "render_safe_html: extracted and cleaned style blocks"
            );
            // Log full first style block content for debugging
            if let Some(first) = cleaned_styles.first() {
                let preview = char_safe_truncate(first, 500);
                debug!(css_preview = %preview, "render_safe_html: first style block preview (first 500 chars)");
            }
        }

        let body_html = extract_body_fragment(&html_without_styles);
        info!(
            body_len = body_html.len(),
            "render_safe_html: extracted body fragment"
        );
        trace!(
            body_preview = char_safe_truncate(&body_html, 300),
            "render_safe_html: body fragment start"
        );

        // Pre-process images before ammonia sanitization
        let preprocessed =
            preprocess_images(&body_html, &mut trackers_blocked, &mut images_blocked);
        info!(
            preprocessed_len = preprocessed.len(),
            trackers = trackers_blocked.len(),
            images_blocked,
            "render_safe_html: image pre-processing complete"
        );
        if preprocessed_len_unchanged(&preprocessed, &body_html) {
            trace!("render_safe_html: preprocessed == body_html (no images modified)");
        }

        // Sanitize with ammonia (style tags are stripped by ammonia, but
        // we re-inject the cleaned CSS afterwards.)
        let sanitizer = build_sanitizer();
        let mut clean_html = sanitizer.clean(&preprocessed).to_string();
        info!(
            pre_len = preprocessed.len(),
            post_len = clean_html.len(),
            "render_safe_html: ammonia sanitization complete"
        );
        // Log first 200 chars before/after to understand what ammonia removed
        let pre_start = char_safe_truncate(&preprocessed, 200);
        let post_start = char_safe_truncate(&clean_html, 200);
        debug!(
            before = %pre_start,
            after = %post_start,
            "render_safe_html: ammonia before/after start"
        );

        // Re-inject cleaned style blocks after sanitization.
        // Wrap the output in a proper HTML document structure so that
        // <style> lives in <head>. This is critical because DOMPurify
        // strips <style> tags from document fragments (no <html>/<head>).
        if !cleaned_styles.is_empty() {
            let styles_joined = cleaned_styles.join("\n");
            clean_html = format!(
                "<html><head><style>\n{}\n</style></head><body>{}</body></html>",
                styles_joined, clean_html
            );
            info!(
                final_len = clean_html.len(),
                "render_safe_html: output wrapped in full HTML doc with style in <head>"
            );
            trace!(
                html_preview = char_safe_truncate(&clean_html, 500),
                "render_safe_html: output start"
            );
        }

        info!(
            output_len = clean_html.len(),
            output_html = %clean_html,
            "render_safe_html: final output HTML"
        );

        RenderedHtml {
            html: clean_html,
            trackers_blocked,
            images_blocked,
        }
    }
}

impl Default for PrivacyGuard {
    fn default() -> Self {
        Self::new()
    }
}

fn extract_body_fragment(raw_html: &str) -> String {
    if !looks_like_html_document(raw_html) {
        return raw_html.to_string();
    }

    if let Some(body_start) = find_ascii_case_insensitive(raw_html, "<body") {
        if let Some(open_end) = find_tag_end(&raw_html[body_start..]) {
            let content_start = body_start + open_end + 1;
            if let Some(close_start) =
                find_ascii_case_insensitive(&raw_html[content_start..], "</body")
            {
                return raw_html[content_start..content_start + close_start].to_string();
            }
            return raw_html[content_start..].to_string();
        }
    }

    strip_head_element(raw_html)
}

fn looks_like_html_document(html: &str) -> bool {
    find_ascii_case_insensitive(html, "<html").is_some()
        || find_ascii_case_insensitive(html, "<head").is_some()
        || find_ascii_case_insensitive(html, "<body").is_some()
}

fn strip_head_element(html: &str) -> String {
    let Some(head_start) = find_ascii_case_insensitive(html, "<head") else {
        return html.to_string();
    };
    let Some(close_start_rel) = find_ascii_case_insensitive(&html[head_start..], "</head") else {
        return html.to_string();
    };
    let close_start = head_start + close_start_rel;
    let Some(close_end_rel) = find_tag_end(&html[close_start..]) else {
        return html.to_string();
    };
    let close_end = close_start + close_end_rel + 1;

    let mut stripped = String::with_capacity(html.len().saturating_sub(close_end - head_start));
    stripped.push_str(&html[..head_start]);
    stripped.push_str(&html[close_end..]);
    stripped
}

fn find_ascii_case_insensitive(haystack: &str, needle: &str) -> Option<usize> {
    haystack
        .to_ascii_lowercase()
        .find(&needle.to_ascii_lowercase())
}

fn find_tag_end(html: &str) -> Option<usize> {
    let mut quote: Option<char> = None;
    for (idx, ch) in html.char_indices() {
        match quote {
            Some(current) if ch == current => quote = None,
            Some(_) => {}
            None if ch == '"' || ch == '\'' => quote = Some(ch),
            None if ch == '>' => return Some(idx),
            None => {}
        }
    }
    None
}

/// Parse a CSS style string and keep only properties from the safe allowlist.
///
/// Images and external URLs in CSS are NEVER blocked. Only XSS vectors
/// (javascript:, vbscript:, data:, expression()) are removed.
fn filter_css_properties(style: &str) -> String {
    const SAFE_PROPERTIES: &[&str] = &[
        "color",
        "background",
        "background-color",
        "box-sizing",
        "font-family",
        "font-size",
        "font-style",
        "font-weight",
        "font-variant",
        "text-align",
        "text-decoration",
        "text-indent",
        "text-transform",
        "line-height",
        "letter-spacing",
        "word-spacing",
        "word-break",
        "overflow-wrap",
        "word-wrap",
        "white-space",
        "vertical-align",
        "direction",
        "margin",
        "margin-top",
        "margin-right",
        "margin-bottom",
        "margin-left",
        "padding",
        "padding-top",
        "padding-right",
        "padding-bottom",
        "padding-left",
        "border",
        "border-top",
        "border-right",
        "border-bottom",
        "border-left",
        "border-color",
        "border-style",
        "border-width",
        "border-collapse",
        "border-spacing",
        "width",
        "max-width",
        "min-width",
        "height",
        "max-height",
        "min-height",
        "display",
        "opacity",
        "overflow",
        "overflow-x",
        "overflow-y",
        "visibility",
        "float",
        "font",
        "clear",
        "position",
        "background-position",
        "background-position-x",
        "background-position-y",
        "list-style",
        "list-style-type",
        "list-style-position",
        "table-layout",
    ];

    let parts: Vec<&str> = style.split(';').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
    let before = parts.len();
    let mut removed_props: Vec<String> = Vec::new();
    let allowed: Vec<String> = parts
        .iter()
        .filter_map(|decl| {
            let colon = decl.find(':')?;
            let prop = decl[..colon].trim().to_lowercase();
            let value = decl[colon + 1..].trim().to_lowercase();
            if !SAFE_PROPERTIES.contains(&prop.as_str()) {
                removed_props.push(format!("{prop} (not in allowlist)"));
                return None;
            }
            // Block XSS vectors (always, regardless of mode)
            if value.contains("expression(")
                || value.contains("javascript:")
                || value.contains("vbscript:")
                || value.contains("data:")
            {
                removed_props.push(format!("{prop}: {value} (XSS vector)"));
                return None;
            }

            // url() and @import are ALLOWED — external images and CSS
            // decorations are never blocked by any privacy mode. Only
            // tracking pixels (<img> with known tracker domains) are
            // blocked separately in preprocess_images.
            Some((*decl).to_string())
        })
        .collect::<Vec<String>>();

    if !removed_props.is_empty() {
        info!(
            before,
            after = allowed.len(),
            removed = removed_props.join("; "),
            "filter_css_properties: inline style properties removed"
        );
    }
    allowed.join("; ")
}

// is_safe_background_shorthand_value, is_hex_color, is_css_color_function
// were removed — background shorthand validation is no longer needed since
// url() in CSS is allowed in all privacy modes.

/// Extract <style> blocks from HTML, clean their CSS content, and return
/// (html_without_style_blocks, vec_of_cleaned_css).
///
/// This allows CSS from <head> to survive ammonia sanitization by extracting
/// it beforehand, cleaning it, and re-injecting it into the body area where
/// ammonia will preserve it (since we add "style" to the allowed tags).
fn extract_and_clean_styles(html: &str) -> (String, Vec<String>) {
    let mut cleaned_styles: Vec<String> = Vec::new();
    let mut result = String::with_capacity(html.len());
    let mut last_end = 0usize;
    let mut idx = 0usize;

    let html_lower = html.to_ascii_lowercase();
    while idx < html.len() {
        // Look for <style ...> or <style>
        let remaining_lower = &html_lower[idx..];
        let style_start = match find_ascii_case_insensitive(remaining_lower, "<style") {
            Some(pos) => idx + pos,
            None => break,
        };

        // Find end of opening tag
        let tag_open = &html[style_start..];
        let tag_end = match find_tag_end(tag_open) {
            Some(end) => style_start + end + 1,
            None => {
                idx = style_start + 6;
                continue;
            }
        };

        // Check if it's actually a closing </style> that was matched
        if tag_open[..tag_end - style_start].contains("</style") {
            idx = tag_end;
            continue;
        }

        // Find </style>
        let after_open = &html_lower[tag_end..];
        let close_pos = match find_ascii_case_insensitive(after_open, "</style") {
            Some(p) => tag_end + p,
            None => {
                idx = tag_end;
                continue;
            }
        };

        // Extract the CSS content between <style> and </style>
        let css_content = &html[tag_end..close_pos];

        // Clean the CSS
        let cleaned = clean_css_content(css_content);
        if !cleaned.is_empty() {
            cleaned_styles.push(cleaned);
        }

        // Add the text before <style> to result
        result.push_str(&html[last_end..style_start]);
        last_end = close_pos + 8; // skip past </style>
        idx = last_end;
    }

    result.push_str(&html[last_end..]);
    (result, cleaned_styles)
}

/// Clean CSS content by removing unsafe constructs while preserving
/// legitimate email CSS rules.
fn clean_css_content(css: &str) -> String {
    // Remove CSS comments
    let mut no_comments = String::with_capacity(css.len());
    let mut in_block_comment = false;
    let mut chars = css.chars().peekable();
    while let Some(ch) = chars.next() {
        if in_block_comment {
            if ch == '*' && chars.peek() == Some(&'/') {
                chars.next();
                in_block_comment = false;
            }
            continue;
        }
        if ch == '/' && chars.peek() == Some(&'*') {
            chars.next();
            in_block_comment = true;
            continue;
        }
        no_comments.push(ch);
    }
    trace!(
        css_len_before = css.len(),
        css_len_after = no_comments.len(),
        "clean_css_content: removed CSS comments"
    );

    // Remove @import rules (potential exfiltration vector).
    // Use char_indices for safe UTF-8 advancement.
    let mut result = String::with_capacity(no_comments.len());
    let lowercase = no_comments.to_ascii_lowercase();
    let mut pos = 0usize;
    while pos < no_comments.len() {
        // Check for @import at current position (all ASCII so byte check is safe)
        if lowercase[pos..].starts_with("@import") {
            // Skip past "@import" = 7 ASCII chars
            let mut skip_end = pos + 7;
            // Skip to end of semicolon or block
            let mut in_paren = false;
            while skip_end < no_comments.len() {
                let c = no_comments[skip_end..].chars().next().unwrap_or('\0');
                let c_len = c.len_utf8();
                match c {
                    ';' if !in_paren => {
                        skip_end += c_len;
                        break;
                    }
                    '(' => in_paren = true,
                    ')' => in_paren = false,
                    '{' => {
                        skip_end += c_len;
                        let mut depth = 1u32;
                        while skip_end < no_comments.len() && depth > 0 {
                            let bc = no_comments[skip_end..].chars().next().unwrap_or('\0');
                            match bc {
                                '{' => depth += 1,
                                '}' => depth -= 1,
                                _ => {}
                            }
                            skip_end += bc.len_utf8();
                        }
                        break;
                    }
                    _ => {}
                }
                skip_end += c_len;
            }
            debug!(
                skip_len = skip_end - pos,
                "clean_css_content: removed @import rule"
            );
            pos = skip_end;
        } else {
            let ch = no_comments[pos..].chars().next().unwrap_or('\0');
            result.push(ch);
            pos += ch.len_utf8();
        }
    }
    trace!(
        result_len = result.len(),
        "clean_css_content: @import removal complete"
    );

    // Remove rules containing url(), expression(), javascript:, data:, etc.
    let mut filtered = String::with_capacity(result.len());
    let lower_result = result.to_ascii_lowercase();
    let mut rule_start = 0;
    while rule_start < result.len() {
        let block_start = match result[rule_start..].find('{') {
            Some(i) => rule_start + i,
            None => {
                filtered.push_str(&result[rule_start..]);
                break;
            }
        };
        let block_end = match find_matching_brace(&result, block_start) {
            Some(i) => i + 1,
            None => {
                filtered.push_str(&result[rule_start..]);
                break;
            }
        };

        let rule = &result[rule_start..block_end];
        let lower_rule = &lower_result[rule_start..block_end];

        // Skip rules with dangerous patterns
        let has_dangerous = [
            "url(",
            "expression(",
            "javascript:",
            "vbscript:",
            "data:",
            "\\",
        ]
        .iter()
        .any(|&p| lower_rule.contains(p));

        if !has_dangerous {
            filtered.push_str(rule);
        } else {
            debug!(
                rule = %char_safe_truncate(rule, 100),
                "clean_css_content: removed rule with unsafe content"
            );
        }

        rule_start = block_end;
    }

    let result = filtered.trim().to_string();
    trace!(
        final_len = result.len(),
        "clean_css_content: complete"
    );
    result
}

/// Find the matching closing brace for an opening brace.
fn find_matching_brace(s: &str, open_pos: usize) -> Option<usize> {
    let mut depth = 1u32;
    for (i, &b) in s.as_bytes()[open_pos + 1..].iter().enumerate() {
        match b as char {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(open_pos + 1 + i);
                }
            }
            _ => {}
        }
    }
    None
}

/// Build an ammonia sanitizer configured for safe email HTML rendering.
fn build_sanitizer() -> Builder<'static> {
    let mut builder = Builder::new();

    // Allow safe tags for email HTML
    let tags: HashSet<&'static str> = [
        "a",
        "abbr",
        "b",
        "blockquote",
        "br",
        "code",
        "dd",
        "div",
        "dl",
        "dt",
        "em",
        "h1",
        "h2",
        "h3",
        "h4",
        "h5",
        "h6",
        "hr",
        "i",
        "img",
        "li",
        "ol",
        "p",
        "pre",
        "s",
        "span",
        "strong",
        "sub",
        "sup",
        "table",
        "tbody",
        "td",
        "th",
        "thead",
        "tr",
        "u",
        "ul",
        "center",
        "font",
    ]
    .iter()
    .copied()
    .collect();

    builder.tags(tags);

    // Configure per-tag attributes
    builder.tag_attributes(
        [
            (
                "a",
                ["href", "title", "target"]
                    .iter()
                    .copied()
                    .collect::<HashSet<_>>(),
            ),
            (
                "img",
                ["src", "alt", "width", "height", "class", "data-cid"]
                    .iter()
                    .copied()
                    .collect::<HashSet<_>>(),
            ),
            (
                "td",
                ["colspan", "rowspan", "align", "valign"]
                    .iter()
                    .copied()
                    .collect::<HashSet<_>>(),
            ),
            (
                "th",
                ["colspan", "rowspan", "align", "valign"]
                    .iter()
                    .copied()
                    .collect::<HashSet<_>>(),
            ),
            (
                "table",
                ["border", "cellpadding", "cellspacing", "width", "align"]
                    .iter()
                    .copied()
                    .collect::<HashSet<_>>(),
            ),
            (
                "font",
                ["color", "size", "face"]
                    .iter()
                    .copied()
                    .collect::<HashSet<_>>(),
            ),
            (
                "div",
                ["class", "data-src"]
                    .iter()
                    .copied()
                    .collect::<HashSet<_>>(),
            ),
            (
                "blockquote",
                ["cite"].iter().copied().collect::<HashSet<_>>(),
            ),
        ]
        .iter()
        .cloned()
        .collect(),
    );

    // Generic attributes allowed on all tags
    builder.generic_attributes(
        ["style", "class", "dir", "id"]
            .iter()
            .copied()
            .collect::<HashSet<_>>(),
    );

    // Only allow safe URL schemes (blocks javascript:, data:, vbscript:, etc.)
    builder.url_schemes(
        ["http", "https", "mailto"]
            .iter()
            .copied()
            .collect::<HashSet<_>>(),
    );

    // Add rel="noopener noreferrer" to all links
    builder.link_rel(Some("noopener noreferrer"));

    // Filter style attributes using a CSS property allowlist
    builder.attribute_filter(|_element, attribute, value| {
        if attribute == "style" {
            let filtered = filter_css_properties(value);
            if filtered.is_empty() {
                None
            } else {
                Some(filtered.into())
            }
        } else {
            Some(value.into())
        }
    });

    builder
}

/// Check if the preprocessed output is identical to the input.
/// Used for logging to determine if image preprocessing modified the HTML.
fn preprocessed_len_unchanged(preprocessed: &str, original: &str) -> bool {
    preprocessed.len() == original.len()
}

/// Pre-process img tags before ammonia to handle tracking pixels and privacy modes.
///
/// Uses lol_html (a streaming HTML rewriter) to parse `<img>` elements
/// properly, avoiding the pitfalls of hand-rolled string scanning (attribute
/// quoting, whitespace variations, encoding tricks).
fn preprocess_images(
    html: &str,
    trackers_blocked: &mut Vec<TrackerInfo>,
    images_blocked: &mut u32,
) -> String {
    use std::cell::RefCell;

    // Wrap mutable references in RefCell so they can be captured by the
    // closure passed to lol_html (which requires 'static-compatible FnMut).
    let trackers = RefCell::new(trackers_blocked);
    let blocked = RefCell::new(images_blocked);

    let result = lol_html::rewrite_str(
        html,
        lol_html::RewriteStrSettings {
            element_content_handlers: vec![lol_html::element!("img", |el| {
                let src = el.get_attribute("src");
                let width = el.get_attribute("width");
                let height = el.get_attribute("height");

                // Handle cid: (inline embedded) images: convert src="cid:xxx"
                // to data-cid="xxx" so the frontend can replace with data URIs.
                // ammonia strips cid: URLs since they aren't in allowed schemes.
                if let Some(ref src_val) = src {
                    if let Some(cid) = src_val.strip_prefix("cid:") {
                        let cid_clean = cid.trim_matches(|c| c == '<' || c == '>');
                        el.set_attribute("data-cid", cid_clean).unwrap();
                        el.remove_attribute("src");
                        return Ok(());
                    }
                }

                let action = process_img_tag(
                    src.as_deref(),
                    width.as_deref(),
                    height.as_deref(),
                    &mut trackers.borrow_mut(),
                    &mut blocked.borrow_mut(),
                );

                match action {
                    ImgAction::Remove => {
                        el.remove();
                    }
                    ImgAction::Keep => { /* leave element untouched */ }
                }

                Ok(())
            })],
            ..lol_html::RewriteStrSettings::default()
        },
    );

    match result {
        Ok(rewritten) => rewritten,
        Err(_) => {
            // If the rewriter fails on malformed input, fall through to
            // ammonia which will strip the problematic markup anyway.
            html.to_string()
        }
    }
}

enum ImgAction {
    Remove,
    Keep,
}

fn process_img_tag(
    src: Option<&str>,
    width: Option<&str>,
    height: Option<&str>,
    trackers_blocked: &mut Vec<TrackerInfo>,
    _images_blocked: &mut u32,
) -> ImgAction {
    // External images are NEVER blocked by any privacy mode.
    // Only tracking pixels and known tracker domains are blocked.

    // Tracking pixels are always blocked
    if is_tracking_pixel(width, height) {
        let domain = src.and_then(extract_domain_from_url).unwrap_or_default();
        trackers_blocked.push(TrackerInfo {
            domain,
            tracker_type: "pixel".to_string(),
        });
        return ImgAction::Remove;
    }

    // Known tracker domains are always blocked
    if let Some(src_val) = src {
        if let Some(domain) = extract_domain_from_url(src_val) {
            if is_known_tracker(&domain) {
                trackers_blocked.push(TrackerInfo {
                    domain,
                    tracker_type: "domain".to_string(),
                });
                return ImgAction::Remove;
            }
        }
    }

    ImgAction::Keep
}

/// Extract the domain from a URL, stripping protocol and path.
fn extract_domain_from_url(url: &str) -> Option<String> {
    let without_protocol = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);

    let domain = without_protocol
        .split('/')
        .next()
        .unwrap_or(without_protocol);
    if domain.is_empty() {
        None
    } else {
        Some(domain.to_string())
    }
}

/// Escape special HTML characters for use in attribute values.
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn linkify_html_text_nodes(html: &str) -> String {
    use lol_html::html_content::ContentType;
    use std::cell::RefCell;
    use std::rc::Rc;

    let anchor_depth = Rc::new(RefCell::new(0usize));
    let anchor_depth_for_element = Rc::clone(&anchor_depth);
    let anchor_depth_for_text = Rc::clone(&anchor_depth);

    lol_html::rewrite_str(
        html,
        lol_html::RewriteStrSettings {
            element_content_handlers: vec![lol_html::element!("a", move |el| {
                *anchor_depth_for_element.borrow_mut() += 1;
                let anchor_depth = Rc::clone(&anchor_depth_for_element);
                if let Some(handlers) = el.end_tag_handlers() {
                    handlers.push(Box::new(move |_| {
                        let mut depth = anchor_depth.borrow_mut();
                        *depth = depth.saturating_sub(1);
                        Ok(())
                    }));
                }
                Ok(())
            })],
            document_content_handlers: vec![lol_html::doc_text!(move |text| {
                if *anchor_depth_for_text.borrow() == 0 {
                    if let Some(linked) = linkify_text_to_html(text.as_str()) {
                        text.replace(&linked, ContentType::Html);
                    }
                }
                Ok(())
            })],
            ..lol_html::RewriteStrSettings::default()
        },
    )
    .unwrap_or_else(|_| html.to_string())
}

fn linkify_text_to_html(text: &str) -> Option<String> {
    let mut output = String::new();
    let mut last_copied = 0usize;
    let mut index = 0usize;
    let mut changed = false;

    while index < text.len() {
        if starts_with_http_url(text, index) {
            let raw_end = scan_url_end(text, index);
            let link_end = trim_url_end(text, raw_end);
            if link_end > index {
                output.push_str(&html_escape(&text[last_copied..index]));
                append_anchor(&mut output, &text[index..link_end], &text[index..link_end]);
                last_copied = link_end;
                index = link_end;
                changed = true;
                continue;
            }
        }

        if let Some(email_end) = scan_email_end(text, index) {
            let email = &text[index..email_end];
            output.push_str(&html_escape(&text[last_copied..index]));
            append_anchor(&mut output, &format!("mailto:{email}"), email);
            last_copied = email_end;
            index = email_end;
            changed = true;
            continue;
        }

        index = next_char_index(text, index);
    }

    if changed {
        output.push_str(&html_escape(&text[last_copied..]));
        Some(output)
    } else {
        None
    }
}

fn append_anchor(output: &mut String, href: &str, label: &str) {
    output.push_str(r#"<a href=""#);
    output.push_str(&html_escape(href));
    output.push_str(r#"" target="_blank" rel="noopener noreferrer">"#);
    output.push_str(&html_escape(label));
    output.push_str("</a>");
}

fn starts_with_http_url(text: &str, index: usize) -> bool {
    text[index..].starts_with("http://") || text[index..].starts_with("https://")
}

fn scan_url_end(text: &str, start: usize) -> usize {
    let mut end = start;
    for (offset, ch) in text[start..].char_indices() {
        if ch.is_whitespace() || matches!(ch, '<' | '>' | '"' | '\'') {
            break;
        }
        end = start + offset + ch.len_utf8();
    }
    end
}

fn trim_url_end(text: &str, mut end: usize) -> usize {
    while let Some(ch) = text[..end].chars().last() {
        if matches!(ch, '.' | ',' | '!' | '?' | ':' | ';' | ')' | ']' | '}') {
            end -= ch.len_utf8();
        } else {
            break;
        }
    }
    end
}

fn scan_email_end(text: &str, start: usize) -> Option<usize> {
    if start > 0 {
        let previous = text[..start].chars().last()?;
        if is_email_local_char(previous) || previous == '@' {
            return None;
        }
    }

    let mut index = start;
    let mut local_len = 0usize;
    while index < text.len() {
        let ch = text[index..].chars().next()?;
        if !is_email_local_char(ch) {
            break;
        }
        local_len += ch.len_utf8();
        index += ch.len_utf8();
    }

    if local_len == 0 || !text[index..].starts_with('@') {
        return None;
    }
    index += 1;

    let domain_start = index;
    let mut has_dot = false;
    while index < text.len() {
        let ch = text[index..].chars().next()?;
        if !is_email_domain_char(ch) {
            break;
        }
        if ch == '.' {
            has_dot = true;
        }
        index += ch.len_utf8();
    }

    while index > domain_start {
        let ch = text[..index].chars().last()?;
        if matches!(ch, '.' | '-') {
            index -= ch.len_utf8();
        } else {
            break;
        }
    }

    let domain = &text[domain_start..index];
    if !has_dot || !domain_has_valid_labels(domain) {
        return None;
    }

    Some(index)
}

fn domain_has_valid_labels(domain: &str) -> bool {
    let mut labels = domain.split('.');
    let Some(first) = labels.next() else {
        return false;
    };
    if first.is_empty() {
        return false;
    }
    let mut saw_tld = false;
    for label in labels {
        if label.is_empty() {
            return false;
        }
        saw_tld = true;
        if label.len() >= 2 && label.chars().all(|ch| ch.is_ascii_alphabetic()) {
            return true;
        }
    }
    saw_tld
}

fn is_email_local_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '%' | '+' | '-')
}

fn is_email_domain_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-')
}

fn next_char_index(text: &str, index: usize) -> usize {
    index
        + text[index..]
            .chars()
            .next()
            .map(char::len_utf8)
            .unwrap_or(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_removes_script_tags() {
        let guard = PrivacyGuard::new();
        let html = "<p>Hello</p><script>alert('xss')</script><p>World</p>";
        let result = guard.render_safe_html(html, &PrivacyMode::Strict);
        assert!(!result.html.contains("script"));
        assert!(!result.html.contains("alert"));
        assert!(result.html.contains("Hello"));
        assert!(result.html.contains("World"));
    }

    #[test]
    fn test_removes_event_handlers() {
        let guard = PrivacyGuard::new();
        let html = r#"<p onmouseover="alert(1)">Hello</p>"#;
        let result = guard.render_safe_html(html, &PrivacyMode::Strict);
        assert!(!result.html.contains("onmouseover"));
        assert!(result.html.contains("Hello"));
    }

    #[test]
    fn test_blocks_javascript_urls() {
        let guard = PrivacyGuard::new();
        let html = r#"<a href="javascript:alert(1)">Click me</a>"#;
        let result = guard.render_safe_html(html, &PrivacyMode::Strict);
        assert!(!result.html.contains("javascript:"));
        assert!(result.html.contains("Click me"));
    }

    #[test]
    fn test_removes_iframe_tags() {
        let guard = PrivacyGuard::new();
        let html = r#"<p>Before</p><iframe src="https://evil.com">content</iframe><p>After</p>"#;
        let result = guard.render_safe_html(html, &PrivacyMode::Strict);
        assert!(!result.html.contains("iframe"));
        assert!(result.html.contains("Before"));
        assert!(result.html.contains("After"));
    }

    #[test]
    fn test_preserves_safe_style_tags() {
        let guard = PrivacyGuard::new();
        let html = "<p>Hello</p><style>body { color: red; }</style><p>World</p>";
        let result = guard.render_safe_html(html, &PrivacyMode::Strict);
        // Safe CSS in <style> blocks is now preserved for email rendering
        assert!(result.html.contains("<style>"));
        assert!(result.html.contains("color: red"));
        assert!(result.html.contains("Hello"));
        assert!(result.html.contains("World"));
    }

    #[test]
    fn test_blocks_tracking_pixel() {
        let guard = PrivacyGuard::new();
        let html = r#"<p>Hello</p><img src="https://tracker.example.com/pixel.gif" width="1" height="1"><p>World</p>"#;
        let result = guard.render_safe_html(html, &PrivacyMode::Strict);
        assert!(!result.html.contains("tracker.example.com"));
        assert_eq!(result.trackers_blocked.len(), 1);
        assert_eq!(result.trackers_blocked[0].tracker_type, "pixel");
    }

    #[test]
    fn test_blocks_known_tracker_domain() {
        let guard = PrivacyGuard::new();
        let html = r#"<p>Hello</p><img src="https://tracking.mailchimp.com/open.gif" width="100" height="50"><p>World</p>"#;
        let result = guard.render_safe_html(html, &PrivacyMode::Strict);
        assert!(!result.html.contains("mailchimp.com"));
        assert_eq!(result.trackers_blocked.len(), 1);
        assert_eq!(result.trackers_blocked[0].tracker_type, "domain");
    }

    #[test]
    fn test_allows_external_images_in_all_modes() {
        let guard = PrivacyGuard::new();
        let html = r#"<p>Hello</p><img src="https://example.com/photo.jpg"><p>World</p>"#;
        // External images are NEVER blocked by any privacy mode
        let result = guard.render_safe_html(html, &PrivacyMode::Strict);
        assert!(result.html.contains("https://example.com/photo.jpg"));
        assert_eq!(result.images_blocked, 0);
        let result = guard.render_safe_html(html, &PrivacyMode::Normal);
        assert!(result.html.contains("https://example.com/photo.jpg"));
        assert_eq!(result.images_blocked, 0);
        let result = guard.render_safe_html(html, &PrivacyMode::Off);
        assert!(result.html.contains("https://example.com/photo.jpg"));
        assert_eq!(result.images_blocked, 0);
    }

    #[test]
    fn test_still_blocks_trackers_in_all_modes() {
        let guard = PrivacyGuard::new();
        let html = r#"<img src="https://tracking.mailchimp.com/open.gif" width="100" height="50">"#;
        let result = guard.render_safe_html(html, &PrivacyMode::Normal);
        assert!(!result.html.contains("mailchimp.com"));
        assert_eq!(result.trackers_blocked.len(), 1);
    }

    #[test]
    fn test_tracking_pixels_always_blocked() {
        let guard = PrivacyGuard::new();
        let html = r#"<p>Hello</p><img src="https://tracker.example.com/pixel.gif" width="1" height="1"><p>World</p>"#;
        let result = guard.render_safe_html(html, &PrivacyMode::Off);
        assert!(!result.html.contains("tracker.example.com"));
        assert_eq!(result.trackers_blocked.len(), 1);
    }

    #[test]
    fn test_known_tracker_domains_always_blocked() {
        let guard = PrivacyGuard::new();
        let html = r#"<p>Hello</p><img src="https://tracking.mailchimp.com/open.gif" width="100" height="50"><p>World</p>"#;
        let result = guard.render_safe_html(html, &PrivacyMode::Off);
        assert!(!result.html.contains("mailchimp.com"));
        assert_eq!(result.trackers_blocked.len(), 1);
    }

    #[test]
    fn test_removes_svg_with_event_handlers() {
        let guard = PrivacyGuard::new();
        let html = r#"<p>Before</p><svg onload="alert(1)"><circle r="10"/></svg><p>After</p>"#;
        let result = guard.render_safe_html(html, &PrivacyMode::Strict);
        assert!(!result.html.contains("onload"));
        assert!(!result.html.contains("svg"));
        assert!(result.html.contains("Before"));
        assert!(result.html.contains("After"));
    }

    #[test]
    fn test_allows_css_url_in_inline_styles() {
        // url() in inline styles is now allowed in all modes.
        // Only tracking pixels (<img>) are blocked by the privacy system.
        let guard = PrivacyGuard::new();
        let html = r#"<p style="background: url('https://cdn.example.com/bg.png')">text</p>"#;
        let result = guard.render_safe_html(html, &PrivacyMode::Strict);
        assert!(result.html.contains("cdn.example.com"), "url() should be allowed in Strict mode");
    }

    #[test]
    fn test_allows_safe_css_properties() {
        let guard = PrivacyGuard::new();
        let html = r#"<p style="color: red; font-size: 14px; margin: 10px">text</p>"#;
        let result = guard.render_safe_html(html, &PrivacyMode::Strict);
        assert!(result.html.contains("color: red"));
        assert!(result.html.contains("font-size: 14px"));
    }

    #[test]
    fn test_allows_safe_background_shorthand_for_email_buttons() {
        let guard = PrivacyGuard::new();
        let html = r##"<a style="background: #f38020; color: #ffffff; border: 1px solid #f38020">Open dashboard</a>"##;
        let result = guard.render_safe_html(html, &PrivacyMode::Strict);
        assert!(result.html.contains("background: #f38020"));
        assert!(result.html.contains("color: #ffffff"));
    }

    #[test]
    fn test_preserves_hidden_preheader_clipping_styles() {
        let guard = PrivacyGuard::new();
        let html = r#"<div style="max-width:0px;max-height:0px;overflow-x:hidden;overflow-y:hidden;visibility:hidden;opacity:0">马凯，为您推荐 2 条新动态</div>"#;
        let result = guard.render_safe_html(html, &PrivacyMode::Strict);

        assert!(result.html.contains("max-width:0px"));
        assert!(result.html.contains("max-height:0px"));
        assert!(result.html.contains("overflow-x:hidden"));
        assert!(result.html.contains("overflow-y:hidden"));
        assert!(result.html.contains("visibility:hidden"));
        assert!(result.html.contains("opacity:0"));
    }

    #[test]
    fn render_safe_html_preserves_style_from_head() {
        let guard = PrivacyGuard::new();
        let html = r#"<html><head><title>Leaked subject</title><style>p{color:red}</style></head><body><p>Visible body</p></body></html>"#;
        let result = guard.render_safe_html(html, &PrivacyMode::Strict);

        assert!(result.html.contains("Visible body"));
        assert!(!result.html.contains("Leaked subject"));
        // Safe <style> blocks from <head> are now preserved for email CSS
        assert!(result.html.contains("p{color:red}"));
    }

    #[test]
    fn test_blocks_overlay_properties() {
        let guard = PrivacyGuard::new();
        let html = r#"<div style="position: fixed; top: 0; left: 0; z-index: 9999">overlay</div>"#;
        let result = guard.render_safe_html(html, &PrivacyMode::Strict);
        // position alone is harmless; the dangerous positioning properties
        // (top, left, z-index) are not in the allowlist and are removed.
        assert!(result.html.contains("position"), "position should be allowed");
        assert!(!result.html.contains("z-index"), "z-index should be blocked");
        assert!(!result.html.contains("top:"), "top should be blocked");
        assert!(!result.html.contains("left:"), "left should be blocked");
    }

    #[test]
    fn img_tag_end_respects_quoted_gt() {
        // The alt attribute contains a '>' inside quotes. The naive parser
        // that looks for the first '>' would close the tag early, leaving
        // a stray src=".../pixel.gif" fragment in the output.
        let guard = PrivacyGuard::new();
        let html = r#"<p>Before</p><img alt="hi>there" src="https://tracking.mailchimp.com/open.gif" width="100" height="50"><p>After</p>"#;
        let result = guard.render_safe_html(html, &PrivacyMode::Strict);
        // Tracker must be detected and the src must not survive in output.
        assert!(
            !result.html.contains("mailchimp.com"),
            "tracker src leaked: {}",
            result.html
        );
        assert_eq!(result.trackers_blocked.len(), 1);
    }

    #[test]
    fn extract_attr_does_not_match_substring() {
        // `data-src` should NOT be treated as `src`. A substring-matching
        // parser would pull the data-src value and miss the real src.
        let guard = PrivacyGuard::new();
        let html = r#"<img data-src="https://example.com/local.jpg" src="https://tracking.mailchimp.com/open.gif" width="100" height="50">"#;
        let result = guard.render_safe_html(html, &PrivacyMode::Strict);
        assert!(
            !result.html.contains("mailchimp.com"),
            "tracker leaked: {}",
            result.html
        );
        assert_eq!(
            result.trackers_blocked.len(),
            1,
            "expected real src to be detected"
        );
    }

    #[test]
    fn render_message_html_linkifies_plain_text_urls_and_emails() {
        let guard = PrivacyGuard::new();
        let result = guard.render_message_html(
            "",
            "Visit https://example.com/path and contact support@example.com.",
            &PrivacyMode::Strict,
        );

        assert!(result
            .html
            .contains(r#"<a href="https://example.com/path" target="_blank" rel="noopener noreferrer">https://example.com/path</a>"#));
        assert!(result
            .html
            .contains(r#"<a href="mailto:support@example.com" target="_blank" rel="noopener noreferrer">support@example.com</a>"#));
        assert!(result.html.contains("<pre"));
    }

    #[test]
    fn render_message_html_linkifies_html_text_nodes() {
        let guard = PrivacyGuard::new();
        let result = guard.render_message_html(
            "<p>Open https://example.com or mail team@example.org</p>",
            "",
            &PrivacyMode::Strict,
        );

        assert!(result
            .html
            .contains(r#"<a href="https://example.com" target="_blank" rel="noopener noreferrer">https://example.com</a>"#));
        assert!(result
            .html
            .contains(r#"<a href="mailto:team@example.org" target="_blank" rel="noopener noreferrer">team@example.org</a>"#));
    }

    #[test]
    fn render_message_html_does_not_wrap_existing_links_again() {
        let guard = PrivacyGuard::new();
        let result = guard.render_message_html(
            r#"<p><a href="https://example.com">https://example.com</a> support@example.com</p>"#,
            "",
            &PrivacyMode::Strict,
        );

        assert_eq!(
            result
                .html
                .matches(r#"<a href="https://example.com""#)
                .count(),
            1
        );
        assert!(result
            .html
            .contains(r#"<a href="mailto:support@example.com" target="_blank" rel="noopener noreferrer">support@example.com</a>"#));
    }
}
