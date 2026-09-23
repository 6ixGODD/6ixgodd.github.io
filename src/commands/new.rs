use std::fs;
use std::path::PathBuf;

use time::{Date, OffsetDateTime, macros::format_description};

use crate::error::{Error, IoContext, Result};

pub fn run(title: &str, requested_slug: Option<&str>) -> Result<()> {
    if title.trim().is_empty() || title.contains(['\n', '\r']) {
        return Err(Error::InvalidWorkspace(
            "post title must be one nonempty line".to_owned(),
        ));
    }
    let slug = match requested_slug {
        Some(value) => validate_slug(value)?,
        None => derive_slug(title)?,
    };

    let date = OffsetDateTime::now_utc().date();
    let date_text = format_date(date)?;
    let directory = PathBuf::from(format!("content/posts/{date_text}-{slug}"));
    let source = directory.join("index.rst");

    if source.exists() {
        return Err(Error::InvalidWorkspace(format!(
            "post already exists: {}",
            source.display()
        )));
    }

    fs::create_dir_all(&directory).at(&directory)?;

    let underline = "=".repeat(title.chars().count().max(1));
    let body = format!(
        "{title}\n{underline}\n\n:date: {date_text}\n:slug: {slug}\n:tags:\n:lang: en\n:draft: true\n\n"
    );
    fs::write(&source, body).at(&source)?;

    println!("created {}", source.display());
    Ok(())
}

fn derive_slug(title: &str) -> Result<String> {
    let mut slug = String::new();
    let mut previous_dash = false;

    for ch in title.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            previous_dash = false;
        } else if (ch.is_ascii_whitespace() || ch == '-' || ch == '_')
            && !slug.is_empty()
            && !previous_dash
        {
            slug.push('-');
            previous_dash = true;
        }
    }

    let slug = slug.trim_matches('-').to_owned();
    if slug.is_empty() {
        return Err(Error::InvalidSlug(
            "could not derive an ASCII slug; pass --slug explicitly".to_owned(),
        ));
    }

    validate_slug(&slug)
}

fn validate_slug(value: &str) -> Result<String> {
    let valid = !value.is_empty()
        && !value.starts_with('-')
        && !value.ends_with('-')
        && !value.contains("--")
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-');

    if !valid {
        return Err(Error::InvalidSlug(format!(
            "{value:?}; expected lowercase ASCII letters, digits, and single-purpose '-' separators"
        )));
    }

    Ok(value.to_owned())
}

fn format_date(date: Date) -> Result<String> {
    date.format(&format_description!("[year]-[month]-[day]"))
        .map_err(|error| Error::InvalidWorkspace(format!("failed to format current date: {error}")))
}

#[cfg(test)]
mod tests {
    use super::derive_slug;

    #[test]
    fn derives_ascii_slug() {
        assert_eq!(
            derive_slug("Why I inject HTTP clients").unwrap(),
            "why-i-inject-http-clients"
        );
    }
}
