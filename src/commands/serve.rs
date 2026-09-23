use std::fs;
use std::path::{Component, Path, PathBuf};

use tiny_http::{Header, Response, Server, StatusCode};

use crate::error::{Error, IoContext, Result};

pub fn run(port: u16, site_url: &str) -> Result<()> {
    super::build::run(site_url)?;

    let address = format!("127.0.0.1:{port}");
    let server = Server::http(&address).map_err(|error| Error::Server(error.to_string()))?;
    println!("serving http://{address}");

    for request in server.incoming_requests() {
        let relative = safe_request_path(request.url());
        let path = relative
            .as_deref()
            .map(resolve_dist_path)
            .unwrap_or_else(|| PathBuf::from("dist/__invalid__"));

        if !path.is_file() {
            let response =
                Response::from_string("404 Not Found\n").with_status_code(StatusCode(404));
            let _ = request.respond(response);
            continue;
        }

        let bytes = fs::read(&path).at(&path)?;
        let mut response = Response::from_data(bytes);
        if let Some(content_type) = content_type_for(&path) {
            if let Ok(header) = Header::from_bytes(&b"Content-Type"[..], content_type.as_bytes()) {
                response.add_header(header);
            }
        }
        let _ = request.respond(response);
    }

    Ok(())
}

fn safe_request_path(url: &str) -> Option<PathBuf> {
    let path = url.split('?').next().unwrap_or("/").trim_start_matches('/');
    let candidate = Path::new(path);

    if candidate.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return None;
    }

    Some(candidate.to_path_buf())
}

fn resolve_dist_path(relative: &Path) -> PathBuf {
    let mut path = PathBuf::from("dist");
    path.push(relative);

    if relative.as_os_str().is_empty() || path.is_dir() {
        path.push("index.html");
    }

    path
}

fn content_type_for(path: &Path) -> Option<&'static str> {
    match path.extension().and_then(|value| value.to_str()) {
        Some("html") => Some("text/html; charset=utf-8"),
        Some("css") => Some("text/css; charset=utf-8"),
        Some("js") => Some("text/javascript; charset=utf-8"),
        Some("json") => Some("application/json; charset=utf-8"),
        Some("xml") => Some("application/xml; charset=utf-8"),
        Some("rst") | Some("txt") => Some("text/plain; charset=utf-8"),
        Some("svg") => Some("image/svg+xml"),
        Some("png") => Some("image/png"),
        Some("jpg") | Some("jpeg") => Some("image/jpeg"),
        Some("webp") => Some("image/webp"),
        Some("avif") => Some("image/avif"),
        Some("pdf") => Some("application/pdf"),
        _ => None,
    }
}
