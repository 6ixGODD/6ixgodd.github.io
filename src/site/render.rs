use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::content::{Post, SiteContent, date_text, safe_relative};
use crate::error::{Error, IoContext, Result};
use crate::rst::ast::{Block, Document};
use crate::rst::render::{escape, inline, render_html};

const CSS: &str = "html{font-size:14px}body{max-width:76em;margin:1.15rem auto;padding:0 1rem;font-family:\"Times New Roman\",Times,serif;line-height:1.35}nav,.mono,pre,code,kbd,samp,table,input{font-family:ui-monospace,SFMono-Regular,Menlo,Monaco,Consolas,\"Liberation Mono\",\"Courier New\",monospace}nav{margin-bottom:1rem}pre{overflow-x:auto;padding:.5rem .65rem;border-left:2px solid #bbb;background:#fafafa}code{font-size:.94em}h1{font-size:1.45rem;margin:.45rem 0}h2{font-size:1.08rem;margin:1.15rem 0 .4rem}p,ul,ol,blockquote,table,figure{margin:.65rem 0}blockquote{margin-left:1rem;padding-left:.8rem;border-left:2px solid #bbb}table{border-collapse:collapse;font-size:.92rem}th,td{padding:.25rem .7rem .25rem 0;text-align:left;vertical-align:top}figure{margin-left:0}figcaption{font-size:.9rem;font-style:italic}img{max-width:100%;height:auto}.meta{font-size:.9rem}.math{overflow-x:auto;margin:.8rem 0}input{font-size:.95rem}hr{border:0;border-top:1px solid #bbb}[hidden]{display:none!important}";
const SEARCH_JS: &str = r#"const input = document.querySelector('#search');
const latest = document.querySelector('#latest');
const results = document.querySelector('#results');
let data = [];

function render() {
  const query = input.value.trim().toLowerCase();
  latest.hidden = Boolean(query);
  results.hidden = !query;
  if (!query) return;
  const words = query.split(/\s+/);
  const found = data.filter(post => words.every(word =>
    (post.date + ' ' + post.title + ' ' + post.tags.join(' ')).toLowerCase().includes(word)));
  results.replaceChildren(document.createTextNode('Search results /\n\n'));
  if (!found.length) { results.append('(no matches)'); return; }
  for (const post of found) {
    results.append(post.date + '  ');
    if (post.tags.length) {
      const tag = document.createElement('a');
      tag.href = 'tags/' + encodeURIComponent(post.tags[0]) + '/';
      tag.textContent = post.tags[0];
      results.append(tag);
    }
    results.append('  ');
    const link = document.createElement('a');
    link.href = post.url;
    link.textContent = post.title;
    results.append(link, '\n');
  }
}

input.addEventListener('input', render);
fetch('search-index.json').then(response => response.json()).then(posts => {
  data = posts;
  render();
}).catch(() => {
  if (input.value.trim()) results.replaceChildren(document.createTextNode('Search unavailable. Use archive/ or tags/.'));
});"#;

#[derive(Serialize)]
struct SearchEntry<'a> {
    title: &'a str,
    date: String,
    tags: &'a [String],
    url: String,
}

fn shell(title: &str, lang: &str, depth: usize, body: &str) -> String {
    let root = "../".repeat(depth);
    format!(
        "<!doctype html>\n<html lang=\"{}\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width\"><title>{}</title><style>{CSS}</style></head><body>\n<nav><a href=\"{root}\">Bochen Shen</a> · <a href=\"{root}tags/\">tags</a> · <a href=\"{root}archive/\">archive</a> · <a href=\"{root}resume/\">resume</a> · <a href=\"https://github.com/6ixGODD\">github ↗</a> · <a href=\"mailto:6goddddddd@gmail.com\">email ↗</a></nav>\n{body}\n</body></html>\n",
        escape(lang),
        escape(title)
    )
}

fn write(root: &Path, path: &str, content: &str) -> Result<()> {
    let target = root.join(path);
    fs::create_dir_all(target.parent().expect("output parent")).at(&target)?;
    fs::write(&target, content).at(&target)
}

fn copy(from: &Path, to: &Path) -> Result<()> {
    fs::create_dir_all(to.parent().expect("copy parent")).at(to)?;
    fs::copy(from, to).at(from)?;
    Ok(())
}

fn copy_tree(from: &Path, to: &Path) -> Result<()> {
    let mut entries = fs::read_dir(from)
        .at(from)?
        .collect::<std::io::Result<Vec<_>>>()
        .at(from)?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let source = entry.path();
        if entry.file_name() == ".gitkeep" {
            continue;
        }
        let name = PathBuf::from(entry.file_name());
        if !safe_relative(&name) {
            return Err(Error::InvalidWorkspace(format!(
                "unsafe static path: {}",
                source.display()
            )));
        }
        let target = to.join(name);
        let kind = entry.file_type().at(&source)?;
        if kind.is_symlink() {
            return Err(Error::InvalidWorkspace(format!(
                "static symlink is not allowed: {}",
                source.display()
            )));
        }
        if kind.is_dir() {
            fs::create_dir_all(&target).at(&target)?;
            copy_tree(&source, &target)?;
        } else if kind.is_file() {
            if target.exists() {
                return Err(Error::InvalidWorkspace(format!(
                    "static file collides with generated output: {}",
                    target.display()
                )));
            }
            copy(&source, &target)?;
        }
    }
    Ok(())
}

fn post_link(post: &Post, prefix: &str) -> String {
    format!("{prefix}posts/{}/", post.meta.slug)
}

fn rows(posts: &[&Post], prefix: &str) -> String {
    let mut out = String::new();
    for post in posts {
        out.push_str(&date_text(post.meta.date));
        out.push_str("  ");
        if let Some(tag) = post.meta.tags.first() {
            out.push_str(&format!(
                "<a href=\"{prefix}tags/{tag}/\">{}</a>",
                escape(tag)
            ));
        }
        out.push_str("  <a href=\"");
        out.push_str(&post_link(post, prefix));
        out.push_str("\">");
        out.push_str(&escape(&post.document.title));
        out.push_str("</a>\n");
    }
    out
}

fn resume_html(document: &Document) -> String {
    let mut out = String::new();
    for block in &document.blocks {
        if let Block::Fields(fields) = block {
            out.push_str("<p>");
            for (i, (_, value)) in fields.iter().enumerate() {
                if i > 0 {
                    out.push_str("<br>");
                }
                out.push_str(&inline(value));
            }
            out.push_str("</p>\n");
        } else {
            out.push_str(&render_html(&Document {
                title: document.title.clone(),
                blocks: vec![block.clone()],
            }));
        }
    }
    out
}

pub fn build(content: &SiteContent, site_url: &str) -> Result<()> {
    if !site_url.starts_with("https://")
        || !site_url.ends_with('/')
        || site_url
            .chars()
            .any(|ch| !(ch.is_ascii_alphanumeric() || matches!(ch, ':' | '/' | '.' | '-' | '_')))
    {
        return Err(Error::InvalidWorkspace(
            "--site-url must be a simple https URL ending in /".to_owned(),
        ));
    }
    let out = Path::new("dist.tmp");
    if out
        .symlink_metadata()
        .is_ok_and(|meta| meta.file_type().is_symlink())
    {
        return Err(Error::InvalidWorkspace(
            "dist.tmp cannot be a symlink".to_owned(),
        ));
    }
    if out.exists() {
        fs::remove_dir_all(out).at(out)?;
    }
    fs::create_dir_all(out).at(out)?;
    let published: Vec<_> = content.posts.iter().filter(|p| !p.meta.draft).collect();
    let latest: Vec<_> = published.iter().take(20).copied().collect();
    let mut home = format!(
        "<h1>{}</h1>\n{}<hr>\n<label class=\"mono\" for=\"search\">search: </label> <input id=\"search\" type=\"search\" size=\"28\" autocomplete=\"off\" placeholder=\"title, tag, date\">\n<pre id=\"latest\">Index of /\n\n{}\nolder: <a href=\"archive/\">archive/</a></pre><pre id=\"results\" hidden></pre>\n<p class=\"mono\"><a href=\"feed.xml\">feed</a> · <a href=\"sitemap.xml\">sitemap</a></p><script src=\"search.js\" defer></script>",
        escape(&content.summary.title),
        render_html(&content.summary),
        rows(&latest, "")
    );
    if published.is_empty() {
        home = home.replace("Index of /\n\n", "Index of /\n\n(no published posts)\n");
    }
    write(
        out,
        "index.html",
        &shell(&content.summary.title, "en", 0, &home),
    )?;
    write(out, "search.js", SEARCH_JS)?;
    let search: Vec<_> = published
        .iter()
        .map(|p| SearchEntry {
            title: &p.document.title,
            date: date_text(p.meta.date),
            tags: &p.meta.tags,
            url: post_link(p, ""),
        })
        .collect();
    write(
        out,
        "search-index.json",
        &serde_json::to_string_pretty(&search)
            .map_err(|e| Error::InvalidWorkspace(e.to_string()))?,
    )?;
    let has_pdf = Path::new("content/resume.pdf").is_file();
    let pdf_link = if has_pdf {
        " · <a href=\"../resume.pdf\">pdf</a>"
    } else {
        ""
    };
    let resume = format!(
        "<article><header><h1>{}</h1><p class=\"meta mono\"><a href=\"resume.txt\">raw</a>{pdf_link}</p></header>{}<p class=\"mono\">[end]</p></article>",
        escape(&content.resume.title),
        resume_html(&content.resume)
    );
    write(
        out,
        "resume/index.html",
        &shell("Resume — Bochen Shen", "en", 1, &resume),
    )?;
    copy(
        Path::new("content/resume.rst"),
        &out.join("resume/resume.rst"),
    )?;
    copy(
        Path::new("content/resume.rst"),
        &out.join("resume/resume.txt"),
    )?;
    if has_pdf {
        copy(Path::new("content/resume.pdf"), &out.join("resume.pdf"))?;
    }
    let mut tags: BTreeMap<&str, Vec<&Post>> = BTreeMap::new();
    let mut years: BTreeMap<i32, Vec<&Post>> = BTreeMap::new();
    let mut months: BTreeMap<(i32, u8), Vec<&Post>> = BTreeMap::new();
    for post in &published {
        let slug = &post.meta.slug;
        let mut meta = escape(&date_text(post.meta.date));
        for tag in &post.meta.tags {
            meta.push_str(&format!(" · <a href=\"../../tags/{tag}/\">{tag}</a>"));
            tags.entry(tag).or_default().push(post);
        }
        meta.push_str(" · <a href=\"index.txt\">raw</a>");
        let body = format!(
            "<article><header><h1>{}</h1><p class=\"meta mono\">{meta}</p></header>{}<p class=\"mono\">[end]</p></article>",
            escape(&post.document.title),
            render_html(&post.document)
        );
        write(
            out,
            &format!("posts/{slug}/index.html"),
            &shell(&post.document.title, &post.meta.lang, 2, &body),
        )?;
        copy(&post.source, &out.join(format!("posts/{slug}/index.rst")))?;
        copy(&post.source, &out.join(format!("posts/{slug}/index.txt")))?;
        for asset in &post.assets {
            let relative = asset
                .strip_prefix(post.source.parent().unwrap())
                .expect("asset under post");
            copy(asset, &out.join("posts").join(slug).join(relative))?;
        }
        let year = post.meta.date.year();
        let month = u8::from(post.meta.date.month());
        years.entry(year).or_default().push(post);
        months.entry((year, month)).or_default().push(post);
    }
    let mut tag_index = String::from("<h1>Index of /tags/</h1><pre>");
    for (tag, posts) in &tags {
        tag_index.push_str(&format!("<a href=\"{tag}/\">{tag}/</a>  {}\n", posts.len()));
        let body = format!(
            "<h1>Index of /tags/{tag}/</h1><pre>{}</pre>",
            rows(posts, "../../")
        );
        write(
            out,
            &format!("tags/{tag}/index.html"),
            &shell(tag, "en", 2, &body),
        )?;
    }
    tag_index.push_str("</pre>");
    write(out, "tags/index.html", &shell("Tags", "en", 1, &tag_index))?;
    let mut archive = String::from("<h1>Index of /archive/</h1><pre>");
    for (year, posts) in years.iter().rev() {
        archive.push_str(&format!(
            "<a href=\"{year}/\">{year}/</a>  {}\n",
            posts.len()
        ));
        let mut year_body = format!("<h1>Index of /archive/{year}/</h1><pre>");
        for ((y, month), month_posts) in months.iter().rev() {
            if y == year {
                year_body.push_str(&format!(
                    "<a href=\"{month:02}/\">{month:02}/</a>  {}\n",
                    month_posts.len()
                ));
                let body = format!(
                    "<h1>Index of /archive/{year}/{month:02}/</h1><pre>{}</pre>",
                    rows(month_posts, "../../../")
                );
                write(
                    out,
                    &format!("archive/{year}/{month:02}/index.html"),
                    &shell("Archive", "en", 3, &body),
                )?;
            }
        }
        year_body.push_str("</pre>");
        write(
            out,
            &format!("archive/{year}/index.html"),
            &shell("Archive", "en", 2, &year_body),
        )?;
    }
    archive.push_str("</pre>");
    write(
        out,
        "archive/index.html",
        &shell("Archive", "en", 1, &archive),
    )?;
    let base = site_url.trim_end_matches('/');
    let mut feed = format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?><feed xmlns=\"http://www.w3.org/2005/Atom\"><title>Bochen Shen</title><id>{base}/</id><link href=\"{base}/feed.xml\" rel=\"self\"/><updated>{}T00:00:00Z</updated>",
        published
            .first()
            .map(|p| date_text(p.meta.date))
            .unwrap_or_else(|| "1970-01-01".to_owned())
    );
    for post in published.iter().take(20) {
        let url = format!("{base}/posts/{}/", post.meta.slug);
        feed.push_str(&format!("<entry><title>{}</title><id>{url}</id><link href=\"{url}\"/><updated>{}T00:00:00Z</updated><summary>{}</summary></entry>", escape(&post.document.title), date_text(post.meta.date), escape(&post.document.title)));
    }
    feed.push_str("</feed>\n");
    write(out, "feed.xml", &feed)?;
    let mut sitemap = String::from(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?><urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">",
    );
    for path in ["", "resume/", "tags/", "archive/"] {
        sitemap.push_str(&format!("<url><loc>{base}/{path}</loc></url>"));
    }
    for tag in tags.keys() {
        sitemap.push_str(&format!("<url><loc>{base}/tags/{tag}/</loc></url>"));
    }
    for year in years.keys() {
        sitemap.push_str(&format!("<url><loc>{base}/archive/{year}/</loc></url>"));
    }
    for (year, month) in months.keys() {
        sitemap.push_str(&format!(
            "<url><loc>{base}/archive/{year}/{month:02}/</loc></url>"
        ));
    }
    for post in &published {
        sitemap.push_str(&format!(
            "<url><loc>{base}/posts/{}/</loc></url>",
            post.meta.slug
        ));
    }
    sitemap.push_str("</urlset>\n");
    write(out, "sitemap.xml", &sitemap)?;
    copy_tree(Path::new("static"), out)?;
    write(out, ".nojekyll", "")?;
    let dist = Path::new("dist");
    if dist
        .symlink_metadata()
        .is_ok_and(|meta| meta.file_type().is_symlink())
    {
        return Err(Error::InvalidWorkspace(
            "dist cannot be a symlink".to_owned(),
        ));
    }
    if dist.exists() {
        fs::remove_dir_all(dist).at(dist)?;
    }
    fs::rename(out, dist).at(out)?;
    Ok(())
}
