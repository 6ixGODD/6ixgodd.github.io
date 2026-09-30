pub mod ast;
pub mod parser;
pub mod render;

/// Convert math with aliases missing from the converter's command vocabulary.
pub fn mathml(expression: &str, style: latex2mathml::DisplayStyle) -> Result<String, String> {
    let mut normalized = String::new();
    let mut chars = expression.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            let mut command = String::new();
            while chars.peek().is_some_and(|c| c.is_ascii_alphabetic()) {
                command.push(chars.next().unwrap());
            }
            match command.as_str() {
                "le" => normalized.push_str("\\leq "),
                "leadsto" => normalized.push('⇝'),
                "deg" => normalized.push_str("\\operatorname{deg}"),
                _ => {
                    normalized.push(ch);
                    normalized.push_str(&command);
                }
            }
        } else {
            normalized.push(ch);
        }
    }
    // The converter implements multiline left-aligned matrices as `align`.
    let normalized = normalized
        .replace("\\begin{aligned}", "\\begin{align}")
        .replace("\\end{aligned}", "\\end{align}");
    let rendered = latex2mathml::latex_to_mathml(&normalized, style).map_err(|e| e.to_string())?;
    if rendered.contains("PARSE ERROR") {
        return Err("unsupported math command".into());
    }
    Ok(rendered)
}

#[cfg(test)]
mod tests {
    #[test]
    fn supports_aligned_multiline_equations() {
        let rendered = super::mathml(
            r"\begin{aligned}a&=b+1,\\c&=d+2.\end{aligned}",
            latex2mathml::DisplayStyle::Block,
        )
        .unwrap();
        assert!(rendered.contains("<mtable"));
        assert_eq!(rendered.matches("<mtr>").count(), 2);
        assert!(!rendered.contains("PARSE ERROR"));
    }
}
