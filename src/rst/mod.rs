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
    let rendered = latex2mathml::latex_to_mathml(&normalized, style).map_err(|e| e.to_string())?;
    if rendered.contains("PARSE ERROR") {
        return Err("unsupported math command".into());
    }
    Ok(rendered)
}
