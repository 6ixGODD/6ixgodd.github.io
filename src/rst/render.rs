use crate::rst::ast::{Block, Document};

pub fn escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    out
}

pub fn inline(value: &str) -> String {
    let mut out = String::new();
    let mut rest = value;
    while !rest.is_empty() {
        if let Some(after) = rest.strip_prefix("``") {
            if let Some(end) = after.find("``") {
                out.push_str("<code>");
                out.push_str(&escape(&after[..end]));
                out.push_str("</code>");
                rest = &after[end + 2..];
                continue;
            }
        }
        if let Some(after) = rest.strip_prefix("**") {
            if let Some(end) = after.find("**") {
                out.push_str("<strong>");
                out.push_str(&inline(&after[..end]));
                out.push_str("</strong>");
                rest = &after[end + 2..];
                continue;
            }
        }
        if let Some(after) = rest.strip_prefix('*') {
            if let Some(end) = after.find('*') {
                out.push_str("<em>");
                out.push_str(&inline(&after[..end]));
                out.push_str("</em>");
                rest = &after[end + 1..];
                continue;
            }
        }
        if let Some(after) = rest.strip_prefix('`') {
            if let Some(end) = after.find("`_") {
                if let Some((label, url)) = after[..end].rsplit_once(" <") {
                    if let Some(url) = url.strip_suffix('>') {
                        out.push_str("<a href=\"");
                        out.push_str(&escape(url));
                        out.push_str("\">");
                        out.push_str(&escape(label));
                        out.push_str("</a>");
                        rest = &after[end + 2..];
                        continue;
                    }
                }
            }
        }
        if let Some(after) = rest.strip_prefix('[') {
            if let Some(end) = after.find("]_") {
                let label = &after[..end];
                if !label.is_empty()
                    && label
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'-')
                {
                    out.push_str("<a href=\"#ref-");
                    out.push_str(label);
                    out.push_str("\">[");
                    out.push_str(label);
                    out.push_str("]</a>");
                    rest = &after[end + 2..];
                    continue;
                }
            }
        }
        let ch = rest.chars().next().unwrap();
        out.push_str(&escape(&ch.to_string()));
        rest = &rest[ch.len_utf8()..];
    }
    out
}

pub fn render_html(document: &Document) -> String {
    let mut out = String::new();
    for block in &document.blocks {
        match block {
            Block::Paragraph(value) => {
                out.push_str("<p>");
                out.push_str(&inline(value));
                out.push_str("</p>\n");
            }
            Block::Fields(fields) => {
                out.push_str("<dl>\n");
                for (key, value) in fields {
                    out.push_str("<dt>");
                    out.push_str(&escape(key));
                    out.push_str("</dt><dd>");
                    out.push_str(&inline(value));
                    out.push_str("</dd>\n");
                }
                out.push_str("</dl>\n");
            }
            Block::Heading(level, value) => {
                out.push_str(&format!("<h{level}>{}</h{level}>\n", inline(value)));
            }
            Block::List(ordered, items) => {
                let tag = if *ordered { "ol" } else { "ul" };
                out.push_str(&format!("<{tag}>\n"));
                for item in items {
                    out.push_str("<li>");
                    out.push_str(&inline(item));
                    out.push_str("</li>\n");
                }
                out.push_str(&format!("</{tag}>\n"));
            }
            Block::Code(lang, body) => {
                out.push_str("<pre><code");
                if !lang.is_empty() {
                    out.push_str(" class=\"language-");
                    out.push_str(&escape(lang));
                    out.push('"');
                }
                out.push('>');
                out.push_str(&escape(body));
                out.push_str("</code></pre>\n");
            }
            Block::Quote(value) => {
                out.push_str("<blockquote><p>");
                out.push_str(&inline(&value.replace('\n', " ")));
                out.push_str("</p></blockquote>\n");
            }
            Block::Image { src, alt, caption } => {
                out.push_str("<figure><img src=\"");
                out.push_str(&escape(src));
                out.push_str("\" alt=\"");
                out.push_str(&escape(alt));
                out.push_str("\">");
                if let Some(value) = caption {
                    out.push_str("<figcaption>");
                    out.push_str(&inline(value));
                    out.push_str("</figcaption>");
                }
                out.push_str("</figure>\n");
            }
            Block::Table(rows) => {
                out.push_str("<table>\n");
                for (i, row) in rows.iter().enumerate() {
                    out.push_str("<tr>");
                    let tag = if i == 0 { "th" } else { "td" };
                    for cell in row {
                        out.push_str(&format!("<{tag}>{}</{tag}>", inline(cell)));
                    }
                    out.push_str("</tr>\n");
                }
                out.push_str("</table>\n");
            }
            Block::Math(value) => {
                out.push_str("<div class=\"math\">");
                out.push_str(value);
                out.push_str("</div>\n");
            }
            Block::Note(title, body) => {
                out.push_str("<aside class=\"note\">");
                if let Some(title) = title {
                    out.push_str("<p><strong>");
                    out.push_str(&inline(title));
                    out.push_str("</strong></p>");
                }
                for paragraph in body.split("\n\n") {
                    out.push_str("<p>");
                    out.push_str(&inline(&paragraph.replace('\n', " ")));
                    out.push_str("</p>");
                }
                out.push_str("</aside>\n");
            }
            Block::Reference(label, title, url) => {
                out.push_str("<p id=\"ref-");
                out.push_str(label);
                out.push_str("\">[");
                out.push_str(label);
                out.push_str("] <a href=\"");
                out.push_str(&escape(url));
                out.push_str("\">");
                out.push_str(&escape(title));
                out.push_str("</a></p>\n");
            }
        }
    }
    out
}
