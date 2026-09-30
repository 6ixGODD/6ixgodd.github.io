use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_ID: AtomicU64 = AtomicU64::new(0);

fn fixture() -> std::path::PathBuf {
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!("bwww-test-{}-{id}", std::process::id()));
    fs::create_dir_all(root.join("content/posts/2026-09-23-first")).unwrap();
    fs::create_dir_all(root.join("content/posts/2026-09-22-draft")).unwrap();
    fs::create_dir_all(root.join("static")).unwrap();
    fs::create_dir_all(root.join("content/posts/2026-09-23-first/code")).unwrap();
    fs::write(
        root.join("content/posts/2026-09-23-first/code/example.cpp"),
        "int main() {}\n",
    )
    .unwrap();
    fs::write(
        root.join("content/summary.rst"),
        "Bochen Shen\n============\n\nHello.\n",
    )
    .unwrap();
    fs::write(
        root.join("content/resume.rst"),
        "Bochen Shen\n============\n\nSkills\n------\n\n* Rust\n",
    )
    .unwrap();
    fs::write(root.join("content/posts/2026-09-23-first/index.rst"), "First post\n==========\n\n:date: 2026-09-23\n:slug: first\n:tags: rust, web\n\nA **real** post.\n\n.. image:: diagram.svg\n   :alt: Diagram\n").unwrap();
    fs::write(
        root.join("content/posts/2026-09-23-first/diagram.svg"),
        "<svg xmlns=\"http://www.w3.org/2000/svg\"/>",
    )
    .unwrap();
    fs::write(
        root.join("content/posts/2026-09-22-draft/index.rst"),
        "Draft\n=====\n\n:date: 2026-09-22\n:slug: draft\n:tags:\n:draft: true\n\nPrivate.\n",
    )
    .unwrap();
    root
}

fn run(root: &Path, command: &str) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_bwww"))
        .arg(command)
        .current_dir(root)
        .output()
        .unwrap()
}

#[test]
fn builds_published_site_and_hides_drafts() {
    let root = fixture();
    assert!(run(&root, "check").status.success());
    let output = run(&root, "build");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let home = fs::read_to_string(root.join("dist/index.html")).unwrap();
    assert!(home.contains("posts/first/"));
    assert!(home.contains("href=\"archive/\""));
    assert!(!home.contains("posts/draft/"));
    assert!(root.join("dist/posts/first/diagram.svg").exists());
    assert!(root.join("dist/posts/first/index.rst").exists());
    assert!(root.join("dist/posts/first/index.txt").exists());
    assert_eq!(
        fs::read_to_string(root.join("dist/posts/first/code/example.cpp")).unwrap(),
        "int main() {}\n"
    );
    assert!(!root.join("dist/posts/draft").exists());
    assert!(root.join("dist/tags/rust/index.html").exists());
    let post = fs::read_to_string(root.join("dist/posts/first/index.html")).unwrap();
    assert!(post.contains("href=\"../../archive/\""));
    assert!(post.contains("<strong>real</strong>"));
    assert!(post.contains("href=\"index.txt\">raw</a>"));
    assert!(root.join("dist/archive/2026/09/index.html").exists());
    assert!(
        fs::read_to_string(root.join("dist/search-index.json"))
            .unwrap()
            .contains("first")
    );
    assert!(root.join("dist/.nojekyll").exists());
    let resume = fs::read_to_string(root.join("dist/resume/index.html")).unwrap();
    assert!(resume.contains("href=\"resume.txt\">raw</a>"));
    assert_eq!(
        fs::read(root.join("dist/resume/resume.rst")).unwrap(),
        fs::read(root.join("dist/resume/resume.txt")).unwrap()
    );
    assert!(
        fs::read_to_string(root.join("dist/feed.xml"))
            .unwrap()
            .contains("https://6ixgodd.github.io/posts/first/")
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn serves_drafts_without_replacing_public_output() {
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::process::{Child, Stdio};
    use std::time::{Duration, Instant};

    struct Server(Child);
    impl Drop for Server {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }

    let root = fixture();
    assert!(run(&root, "build").status.success());
    fs::write(root.join("dist/keep.txt"), "public output").unwrap();
    let socket = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = socket.local_addr().unwrap();
    drop(socket);
    let _server = Server(
        Command::new(env!("CARGO_BIN_EXE_bwww"))
            .args(["serve", "--drafts", "--port", &address.port().to_string()])
            .current_dir(&root)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut stream = loop {
        if let Ok(stream) = TcpStream::connect(address) {
            break stream;
        }
        assert!(Instant::now() < deadline, "draft server did not start");
        std::thread::sleep(Duration::from_millis(25));
    };
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    stream
        .write_all(b"GET /posts/draft/ HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
        .unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    assert!(response.starts_with("HTTP/1.1 200"));
    assert!(response.contains("Private."));
    assert!(response.contains("draft / 草稿"));
    assert!(!root.join("dist/posts/draft").exists());
    assert_eq!(
        fs::read_to_string(root.join("dist/keep.txt")).unwrap(),
        "public output"
    );
    drop(_server);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn rejects_post_assets_that_replace_generated_pages() {
    let root = fixture();
    fs::write(
        root.join("content/posts/2026-09-23-first/index.html"),
        "override",
    )
    .unwrap();
    let output = run(&root, "check");
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("collides"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn invalid_content_does_not_replace_existing_site() {
    let root = fixture();
    assert!(run(&root, "build").status.success());
    fs::write(
        root.join("content/posts/2026-09-23-first/index.rst"),
        "broken",
    )
    .unwrap();
    let output = run(&root, "build");
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("index.rst:1"));
    assert!(root.join("dist/posts/first/index.html").exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn builds_before_the_first_post_exists() {
    let root = fixture();
    fs::remove_dir_all(root.join("content/posts")).unwrap();
    assert!(run(&root, "check").status.success());
    let output = run(&root, "build");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        fs::read_to_string(root.join("dist/index.html"))
            .unwrap()
            .contains("no published posts")
    );
    fs::remove_dir_all(root).unwrap();
}
