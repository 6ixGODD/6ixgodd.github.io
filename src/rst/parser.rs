use std::path::Path;

use crate::error::{Error, Result};
use crate::rst::ast::{Block, Document};

fn fail(path: &Path, line: usize, message: impl Into<String>) -> Error {
    Error::Source {
        path: path.to_path_buf(),
        line,
        message: message.into(),
    }
}

fn rule(line: &str) -> Option<char> {
    let s = line.trim();
    let ch = s.chars().next()?;
    if s.chars().count() >= 3 && matches!(ch, '=' | '-' | '~' | '^') && s.chars().all(|c| c == ch) {
        Some(ch)
    } else {
        None
    }
}

pub fn parse(path: &Path, source: &str) -> Result<Document> {
    let lines: Vec<&str> = source.lines().collect();
    for (number, line) in lines.iter().enumerate() {
        let mut rest = *line;
        while let Some(start) = rest.find('`') {
            rest = &rest[start + 1..];
            if let Some(end) = rest.find("`_") {
                let link = &rest[..end];
                let target = link
                    .rsplit_once(" <")
                    .and_then(|(_, tail)| tail.strip_suffix('>'))
                    .ok_or_else(|| fail(path, number + 1, "unsupported hyperlink syntax"))?;
                if !safe_href(target) {
                    return Err(fail(
                        path,
                        number + 1,
                        format!("unsafe hyperlink target: {target}"),
                    ));
                }
                rest = &rest[end + 2..];
            } else if let Some(end) = rest.find('`') {
                rest = &rest[end + 1..];
            } else {
                break;
            }
        }
    }
    if lines.len() < 2
        || rule(lines[1]) != Some('=')
        || lines[1].trim().chars().count() < lines[0].trim().chars().count()
        || lines[0].trim().is_empty()
    {
        return Err(fail(
            path,
            1,
            "expected a document title with an '=' underline",
        ));
    }
    let mut blocks = Vec::new();
    let mut i = 2;
    while i < lines.len() {
        if lines[i].trim().is_empty() {
            i += 1;
            continue;
        }
        let line = lines[i];
        if i + 1 < lines.len() {
            if let Some(ch) = rule(lines[i + 1]) {
                if lines[i + 1].trim().chars().count() < line.trim().chars().count() {
                    return Err(fail(path, i + 2, "heading underline is too short"));
                }
                let level = match ch {
                    '-' => 2,
                    '~' => 3,
                    '^' => 4,
                    _ => return Err(fail(path, i + 2, "unexpected document title")),
                };
                blocks.push(Block::Heading(level, line.trim().to_owned()));
                i += 2;
                continue;
            }
        }
        if let Some(rest) = line.strip_prefix(".. ") {
            if let Some(lang) = rest.strip_prefix("code-block::") {
                let lang = lang.trim();
                if !lang.is_empty()
                    && !lang
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
                {
                    return Err(fail(path, i + 1, "invalid code language"));
                }
                i += 1;
                if i < lines.len() && lines[i].trim().is_empty() {
                    i += 1;
                }
                let body = indented(path, &lines, &mut i)?;
                blocks.push(Block::Code(lang.to_owned(), body));
                continue;
            }
            if let Some(src) = rest.strip_prefix("image::") {
                let src = src.trim().to_owned();
                if src.is_empty() {
                    return Err(fail(path, i + 1, "image path is empty"));
                }
                i += 1;
                let mut alt = None;
                let mut caption = None;
                while i < lines.len() && lines[i].starts_with("   :") {
                    let option = lines[i].trim();
                    if let Some(value) = option.strip_prefix(":alt:") {
                        alt = Some(value.trim().to_owned());
                    } else if let Some(value) = option.strip_prefix(":caption:") {
                        caption = Some(value.trim().to_owned());
                    } else {
                        return Err(fail(path, i + 1, "unsupported image option"));
                    }
                    i += 1;
                }
                let alt = alt.ok_or_else(|| fail(path, i, "image requires :alt:"))?;
                blocks.push(Block::Image { src, alt, caption });
                continue;
            }
            if rest.trim() == "math::" {
                i += 1;
                if i < lines.len() && lines[i].trim().is_empty() {
                    i += 1;
                }
                let expression = indented(path, &lines, &mut i)?;
                if expression.chars().any(|ch| {
                    !(ch.is_alphanumeric() || ch.is_whitespace() || "+-=*/()^_,.<>".contains(ch))
                }) {
                    return Err(fail(
                        path,
                        i,
                        "unsupported math syntax; use identifiers, numbers and basic operators",
                    ));
                }
                for (position, ch) in expression.chars().enumerate() {
                    if matches!(ch, '^' | '_')
                        && !expression
                            .chars()
                            .nth(position + 1)
                            .is_some_and(|next| next.is_alphanumeric())
                    {
                        return Err(fail(path, i, "math script needs an identifier or number"));
                    }
                }
                blocks.push(Block::Math(expression));
                continue;
            }
            return Err(fail(path, i + 1, "unsupported directive"));
        }
        if line.starts_with(':') {
            let mut fields = Vec::new();
            while i < lines.len() && lines[i].starts_with(':') {
                let (key, value) = lines[i][1..]
                    .split_once(':')
                    .ok_or_else(|| fail(path, i + 1, "invalid field list entry"))?;
                if key.is_empty() || value.trim().is_empty() {
                    return Err(fail(path, i + 1, "field list entry needs a name and value"));
                }
                fields.push((key.to_owned(), value.trim().to_owned()));
                i += 1;
            }
            blocks.push(Block::Fields(fields));
            continue;
        }
        if is_bullet(line).is_some() {
            let ordered = line.as_bytes()[0].is_ascii_digit();
            let mut items = Vec::new();
            while i < lines.len() {
                if let Some(item) = is_bullet(lines[i]) {
                    if lines[i].as_bytes()[0].is_ascii_digit() != ordered {
                        break;
                    }
                    items.push(item.to_owned());
                    i += 1;
                } else if lines[i].trim().is_empty() {
                    i += 1;
                    if i >= lines.len() || is_bullet(lines[i]).is_none() {
                        break;
                    }
                } else {
                    break;
                }
            }
            blocks.push(Block::List(ordered, items));
            continue;
        }
        if line.starts_with("   ") {
            blocks.push(Block::Quote(indented(path, &lines, &mut i)?));
            continue;
        }
        if line.trim_end().ends_with("::") && !line.trim().starts_with("..") {
            let intro = line.trim_end().trim_end_matches(':').trim();
            if !intro.is_empty() {
                blocks.push(Block::Paragraph(intro.to_owned()));
            }
            i += 1;
            if i < lines.len() && lines[i].trim().is_empty() {
                i += 1;
            }
            blocks.push(Block::Code(String::new(), indented(path, &lines, &mut i)?));
            continue;
        }
        if line.starts_with("+---") {
            let mut rows = Vec::new();
            i += 1;
            while i < lines.len() && (lines[i].starts_with('|') || lines[i].starts_with('+')) {
                if lines[i].starts_with('|') {
                    rows.push(
                        lines[i]
                            .trim_matches('|')
                            .split('|')
                            .map(|s| s.trim().to_owned())
                            .collect(),
                    );
                }
                i += 1;
            }
            if rows.is_empty() {
                return Err(fail(path, i, "table has no rows"));
            }
            if i == 0
                || !lines[i - 1].starts_with('+')
                || rows
                    .iter()
                    .any(|row: &Vec<String>| row.len() != rows[0].len())
            {
                return Err(fail(path, i, "malformed table"));
            }
            blocks.push(Block::Table(rows));
            continue;
        }
        if line.starts_with("..") || line.starts_with('|') || rule(line).is_some() {
            return Err(fail(path, i + 1, "unsupported RST syntax"));
        }
        let mut paragraph = String::new();
        while i < lines.len() && !lines[i].trim().is_empty() {
            if i + 1 < lines.len() && rule(lines[i + 1]).is_some() {
                break;
            }
            if lines[i].starts_with(".. ") || is_bullet(lines[i]).is_some() {
                break;
            }
            if !paragraph.is_empty() {
                paragraph.push(' ');
            }
            paragraph.push_str(lines[i].trim());
            i += 1;
        }
        if paragraph.is_empty() {
            return Err(fail(path, i + 1, "unsupported RST syntax"));
        }
        blocks.push(Block::Paragraph(paragraph));
    }
    Ok(Document {
        title: lines[0].trim().to_owned(),
        blocks,
    })
}

fn safe_href(target: &str) -> bool {
    !target.is_empty()
        && !target.chars().any(|c| c.is_control() || c == '\\')
        && (target.starts_with("https://")
            || target.starts_with("http://")
            || target.starts_with("mailto:")
            || (target.starts_with('#') && !target.starts_with("##"))
            || (!target.starts_with('/') && !target.starts_with("//") && !target.contains(':')))
}

fn is_bullet(line: &str) -> Option<&str> {
    for prefix in ["* ", "- ", "+ "] {
        if let Some(rest) = line.strip_prefix(prefix) {
            return Some(rest);
        }
    }
    let (number, rest) = line.split_once(". ")?;
    if !number.is_empty() && number.bytes().all(|b| b.is_ascii_digit()) {
        Some(rest)
    } else {
        None
    }
}

fn indented(path: &Path, lines: &[&str], i: &mut usize) -> Result<String> {
    let start = *i;
    let mut body = Vec::new();
    while *i < lines.len() {
        if lines[*i].trim().is_empty() {
            body.push("");
            *i += 1;
            continue;
        }
        if let Some(rest) = lines[*i].strip_prefix("   ") {
            body.push(rest);
            *i += 1;
        } else {
            break;
        }
    }
    while body.last() == Some(&"") {
        body.pop();
    }
    if body.is_empty() {
        return Err(fail(path, start + 1, "expected indented content"));
    }
    Ok(body.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::parse;
    use crate::rst::render::render_html;
    use std::path::Path;

    #[test]
    fn renders_supported_blocks_and_escapes_text() {
        let source = "Title\n=====\n\nSection\n-------\n\nA **bold** `link <https://example.com>`_ and ``code``.\n\n* one\n* two\n\n.. code-block:: rust\n\n   let x = 1;\n\n.. math::\n\n   x^2 + y\n";
        let document = parse(Path::new("sample.rst"), source).unwrap();
        let html = render_html(&document);
        assert!(html.contains("<h2>Section</h2>"));
        assert!(html.contains("<strong>bold</strong>"));
        assert!(html.contains("<a href=\"https://example.com\">link</a>"));
        assert!(html.contains("<li>one</li>"));
        assert!(html.contains("<pre><code class=\"language-rust\">let x = 1;</code></pre>"));
        assert!(html.contains("<msup><mi>x</mi><mn>2</mn></msup>"));
    }

    #[test]
    fn rejects_unsupported_or_unsafe_source() {
        assert!(
            parse(
                Path::new("sample.rst"),
                "Title\n=====\n\n.. include:: secret\n"
            )
            .is_err()
        );
        assert!(
            parse(
                Path::new("sample.rst"),
                "Title\n=====\n\n`bad <javascript:alert(1)>`_\n"
            )
            .is_err()
        );
    }
}
