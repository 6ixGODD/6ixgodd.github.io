use std::collections::BTreeSet;
use std::fs;
use std::path::{Component, Path, PathBuf};

use time::{Date, macros::format_description};

use crate::error::{Error, IoContext, Result};
use crate::rst::{
    ast::{Block, Document},
    parser,
};

#[derive(Debug, Clone)]
pub struct PostMeta {
    pub date: Date,
    pub slug: String,
    pub tags: Vec<String>,
    pub lang: String,
    pub draft: bool,
}

#[derive(Debug)]
pub struct Post {
    pub source: PathBuf,
    pub document: Document,
    pub meta: PostMeta,
    pub assets: Vec<PathBuf>,
}

#[derive(Debug)]
pub struct SiteContent {
    pub summary: Document,
    pub resume: Document,
    pub posts: Vec<Post>,
}

fn source_error(path: &Path, line: usize, message: impl Into<String>) -> Error {
    Error::Source {
        path: path.to_path_buf(),
        line,
        message: message.into(),
    }
}

fn read_document(path: &Path) -> Result<Document> {
    if path.symlink_metadata().at(path)?.file_type().is_symlink() {
        return Err(source_error(path, 1, "symlinks are not allowed in content"));
    }
    let source = fs::read_to_string(path).at(path)?;
    parser::parse(path, &source)
}

fn validate_static(path: &Path) -> Result<()> {
    for entry in fs::read_dir(path).at(path)? {
        let entry = entry.at(path)?;
        let child = entry.path();
        let kind = entry.file_type().at(&child)?;
        if kind.is_symlink() {
            return Err(source_error(&child, 1, "static symlinks are not allowed"));
        }
        if kind.is_dir() {
            validate_static(&child)?;
        }
    }
    Ok(())
}

pub fn load() -> Result<SiteContent> {
    for required in [
        "content/summary.rst",
        "content/resume.rst",
        "content/posts",
        "static",
    ] {
        if !Path::new(required).exists() {
            return Err(Error::InvalidWorkspace(format!(
                "required path does not exist: {required}"
            )));
        }
    }
    for root in ["content", "content/posts", "static"] {
        let path = Path::new(root);
        if path.symlink_metadata().at(path)?.file_type().is_symlink() {
            return Err(source_error(path, 1, "source roots cannot be symlinks"));
        }
    }
    let pdf = Path::new("content/resume.pdf");
    if pdf
        .symlink_metadata()
        .is_ok_and(|meta| meta.file_type().is_symlink())
    {
        return Err(source_error(pdf, 1, "resume PDF cannot be a symlink"));
    }
    validate_static(Path::new("static"))?;
    let summary = read_document(Path::new("content/summary.rst"))?;
    let resume = read_document(Path::new("content/resume.rst"))?;
    let mut posts = Vec::new();
    let mut slugs = BTreeSet::new();
    let root = Path::new("content/posts");
    let mut dirs = fs::read_dir(root)
        .at(root)?
        .collect::<std::io::Result<Vec<_>>>()
        .at(root)?;
    dirs.sort_by_key(|entry| entry.file_name());
    for entry in dirs {
        let path = entry.path();
        if entry.file_type().at(&path)?.is_symlink() {
            return Err(source_error(
                &path,
                1,
                "symlinks are not allowed in content",
            ));
        }
        if !entry.file_type().at(&path)?.is_dir() {
            return Err(source_error(&path, 1, "posts must be directories"));
        }
        let source = path.join("index.rst");
        if source
            .symlink_metadata()
            .at(&source)?
            .file_type()
            .is_symlink()
        {
            return Err(source_error(
                &source,
                1,
                "symlinks are not allowed in content",
            ));
        }
        let raw = fs::read_to_string(&source).at(&source)?;
        let (clean, meta) = parse_metadata(&source, &raw)?;
        if !slugs.insert(meta.slug.clone()) {
            return Err(source_error(
                &source,
                1,
                format!("duplicate slug: {}", meta.slug),
            ));
        }
        let document = parser::parse(&source, &clean)?;
        let mut assets = Vec::new();
        for block in &document.blocks {
            if let Block::Image { src, .. } = block {
                if src.starts_with("https://") || src.starts_with("http://") {
                    continue;
                }
                let relative = Path::new(src);
                if !safe_relative(relative) {
                    return Err(source_error(
                        &source,
                        1,
                        format!("unsafe image path: {src}"),
                    ));
                }
                let image = path.join(relative);
                let owner = fs::canonicalize(&path).at(&path)?;
                let resolved = fs::canonicalize(&image)
                    .map_err(|_| source_error(&source, 1, format!("missing image: {src}")))?;
                if !resolved.starts_with(&owner) || !resolved.is_file() {
                    return Err(source_error(
                        &source,
                        1,
                        format!("missing or unsafe image: {src}"),
                    ));
                }
                assets.push(image);
            }
        }
        posts.push(Post {
            source,
            document,
            meta,
            assets,
        });
    }
    posts.sort_by(|a, b| {
        b.meta
            .date
            .cmp(&a.meta.date)
            .then(a.meta.slug.cmp(&b.meta.slug))
    });
    Ok(SiteContent {
        summary,
        resume,
        posts,
    })
}

pub fn safe_relative(path: &Path) -> bool {
    !path.as_os_str().is_empty()
        && path.components().all(|c| matches!(c, Component::Normal(_)))
        && !path.to_string_lossy().contains('\\')
}

pub fn date_text(date: Date) -> String {
    date.format(&format_description!("[year]-[month]-[day]"))
        .expect("valid date")
}

fn parse_metadata(path: &Path, source: &str) -> Result<(String, PostMeta)> {
    let lines: Vec<&str> = source.lines().collect();
    if lines.len() < 2 {
        return Err(source_error(path, 1, "post needs a title"));
    }
    let mut fields = std::collections::BTreeMap::new();
    let mut i = 2;
    while i < lines.len() && lines[i].trim().is_empty() {
        i += 1;
    }
    while i < lines.len() && lines[i].starts_with(':') {
        let (key, value) = lines[i][1..]
            .split_once(':')
            .ok_or_else(|| source_error(path, i + 1, "invalid metadata field"))?;
        if !matches!(key, "date" | "slug" | "tags" | "lang" | "draft") {
            return Err(source_error(
                path,
                i + 1,
                format!("unknown metadata field: {key}"),
            ));
        }
        if fields.insert(key, value.trim()).is_some() {
            return Err(source_error(
                path,
                i + 1,
                format!("duplicate metadata field: {key}"),
            ));
        }
        i += 1;
    }
    let get = |key| {
        fields
            .get(key)
            .copied()
            .ok_or_else(|| source_error(path, 1, format!("missing :{key}:")))
    };
    let date = Date::parse(get("date")?, &format_description!("[year]-[month]-[day]"))
        .map_err(|_| source_error(path, 1, "invalid :date:; expected YYYY-MM-DD"))?;
    let slug = get("slug")?;
    if slug.is_empty()
        || slug.starts_with('-')
        || slug.ends_with('-')
        || !slug
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    {
        return Err(source_error(path, 1, "invalid :slug:"));
    }
    let tags_raw = get("tags")?;
    let tags = if tags_raw.is_empty() {
        Vec::new()
    } else {
        tags_raw
            .split(',')
            .map(str::trim)
            .map(|tag| {
                if tag.is_empty()
                    || !tag
                        .bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
                {
                    Err(source_error(
                        path,
                        1,
                        format!("invalid or empty tag: {tag:?}"),
                    ))
                } else {
                    Ok(tag.to_owned())
                }
            })
            .collect::<Result<Vec<_>>>()?
    };
    let unique: BTreeSet<_> = tags.iter().collect();
    if unique.len() != tags.len() {
        return Err(source_error(path, 1, "duplicate tag"));
    }
    let lang = fields.get("lang").copied().unwrap_or("en");
    if lang.is_empty() || !lang.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-') {
        return Err(source_error(path, 1, "invalid :lang:"));
    }
    let draft = match fields.get("draft").copied().unwrap_or("false") {
        "true" => true,
        "false" => false,
        _ => return Err(source_error(path, 1, ":draft: must be true or false")),
    };
    let mut clean = lines[..2].join("\n");
    clean.push_str("\n\n");
    clean.push_str(&lines[i..].join("\n"));
    Ok((
        clean,
        PostMeta {
            date,
            slug: slug.to_owned(),
            tags,
            lang: lang.to_owned(),
            draft,
        },
    ))
}
