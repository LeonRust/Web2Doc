//! M1.16 集成测试：localhost fixture 文档站端到端 + 续传 + max-pages 截断。
//! 对齐验收 A1 / A3 / A10、S5 / S6 / S9。localhost server 纳入默认 `cargo test`（tasks R-6）。

use std::path::PathBuf;

use url::Url;
use web2doc::cli::Mode;
use web2doc::cli::OutputFormat;
use web2doc::config::Config;
use web2doc::fetcher::StaticFetcher;
use web2doc::pipeline;

const PROSE: &str = "Rust is a systems programming language focused on safety and performance. \
    It guarantees memory safety without a garbage collector by using its ownership model, where \
    each value has a single owner and the compiler enforces strict borrowing rules at compile time. \
    References must always be valid, and the borrow checker rejects programs that could lead to \
    dangling pointers or data races. Ownership can be moved or temporarily shared through immutable \
    and mutable borrows, and these guarantees eliminate entire classes of runtime errors common in \
    other systems languages while still producing fast native machine code with zero-cost abstractions.";

fn doc_html(title: &str, link: &str) -> String {
    format!(
        "<html><head><title>{title}</title></head><body>\
         <nav>site menu noise</nav>\
         <article><h1>{title}</h1><p>{PROSE}</p>\
         <p>See <a href=\"{link}\">other</a> <img src=\"/img/{title}.png\"></p></article>\
         <footer>copyright noise</footer></body></html>"
    )
}

fn sitemap(port: u16) -> String {
    format!(
        "<?xml version=\"1.0\"?><urlset>\
         <url><loc>http://127.0.0.1:{port}/docs/a</loc></url>\
         <url><loc>http://127.0.0.1:{port}/docs/b</loc></url>\
         <url><loc>http://127.0.0.1:{port}/tags/x</loc></url>\
         </urlset>"
    )
}

/// 启动后台 fixture server，返回监听端口。
fn start_server() -> u16 {
    let server = tiny_http::Server::http("127.0.0.1:0").expect("bind");
    let port = server.server_addr().to_ip().expect("ip addr").port();
    std::thread::spawn(move || {
        for req in server.incoming_requests() {
            let path = req.url().split('?').next().unwrap_or("").to_string();
            let (status, body, ctype): (u16, String, &str) = match path.as_str() {
                "/sitemap.xml" => (200, sitemap(port), "application/xml"),
                "/docs/a" => (200, doc_html("A", "/docs/b"), "text/html"),
                "/docs/b" => (200, doc_html("B", "/docs/a"), "text/html"),
                _ => (404, "nope".to_string(), "text/plain"),
            };
            let header =
                tiny_http::Header::from_bytes(&b"Content-Type"[..], ctype.as_bytes()).unwrap();
            let resp = tiny_http::Response::from_string(body)
                .with_status_code(status)
                .with_header(header);
            let _ = req.respond(resp);
        }
    });
    port
}

fn config_for(port: u16, out: PathBuf, max_pages: usize, fresh: bool) -> Config {
    Config {
        start_url: Url::parse(&format!("http://127.0.0.1:{port}/docs/")).unwrap(),
        out_dir: out,
        prefix: None,
        include_prefixes: vec![],
        max_pages,
        concurrency: 4,
        delay_ms: 0,
        mode: Mode::Static,
        chrome_path: None,
        base_url: "http://localhost".to_string(),
        model: "none".to_string(),
        max_failure_rate: 0.2,
        bundle: false,
        format: OutputFormat::Md,
        ignore_robots: false,
        fresh,
        verbose: 0,
        api_key: None,
        proxy: None,
    }
}

#[tokio::test]
async fn end_to_end_static_site() {
    let port = start_server();
    let tmp = tempfile::tempdir().unwrap();
    let config = config_for(port, tmp.path().to_path_buf(), 500, false);
    let fetcher = StaticFetcher::new(None).unwrap();

    let report = pipeline::run(&fetcher, &config).await.unwrap();

    // 覆盖率：2 个文档页（tags/x 被文档页判定过滤），无失败
    assert_eq!(report.ok, 2, "expected 2 written pages");
    assert!(!report.is_failure(0.2));

    // 产物结构（S5）
    assert!(tmp.path().join("index.md").exists());
    assert!(tmp.path().join("manifest.json").exists());
    assert!(tmp.path().join("docs/a.md").exists());
    assert!(tmp.path().join("docs/b.md").exists());

    // 正文 + 标题 + 内链相对化（rewrite→convert）
    let md_a = std::fs::read_to_string(tmp.path().join("docs/a.md")).unwrap();
    assert!(md_a.contains("# A"));
    assert!(
        md_a.contains("(b.md)"),
        "internal link should be relativized, got: {md_a}"
    );

    // 索引列出已写页面
    let idx = std::fs::read_to_string(tmp.path().join("index.md")).unwrap();
    assert!(idx.contains("docs/a.md"));
}

#[tokio::test]
async fn resume_skips_completed_no_half_state() {
    let port = start_server();
    let tmp = tempfile::tempdir().unwrap();
    let config = config_for(port, tmp.path().to_path_buf(), 500, false);
    let fetcher = StaticFetcher::new(None).unwrap();

    let r1 = pipeline::run(&fetcher, &config).await.unwrap();
    assert_eq!(r1.ok, 2);

    // 第二次（续传，非 fresh）：已 Written 不重复处理，产物保持，无半成品（S6）
    let r2 = pipeline::run(&fetcher, &config).await.unwrap();
    assert_eq!(r2.ok, 2);
    assert!(tmp.path().join("docs/a.md").exists());
    assert!(tmp.path().join("docs/b.md").exists());
}

#[tokio::test]
async fn max_pages_truncation_is_partial_not_failure() {
    let port = start_server();
    let tmp = tempfile::tempdir().unwrap();
    let config = config_for(port, tmp.path().to_path_buf(), 1, false);
    let fetcher = StaticFetcher::new(None).unwrap();

    let report = pipeline::run(&fetcher, &config).await.unwrap();

    // baseline 2 > max-pages 1 → Partial，不判失败（A10）
    assert!(report.partial);
    assert_eq!(report.ok, 1);
    assert!(!report.is_failure(0.2));
}

/// 启动含 `robots.txt`（Disallow /docs/b）的 fixture server。
fn start_server_with_robots() -> u16 {
    let server = tiny_http::Server::http("127.0.0.1:0").expect("bind");
    let port = server.server_addr().to_ip().expect("ip addr").port();
    std::thread::spawn(move || {
        for req in server.incoming_requests() {
            let path = req.url().split('?').next().unwrap_or("").to_string();
            let (status, body, ctype): (u16, String, &str) = match path.as_str() {
                "/robots.txt" => (
                    200,
                    "User-agent: *\nDisallow: /docs/b".to_string(),
                    "text/plain",
                ),
                "/sitemap.xml" => (200, sitemap(port), "application/xml"),
                "/docs/a" => (200, doc_html("A", "/docs/b"), "text/html"),
                "/docs/b" => (200, doc_html("B", "/docs/a"), "text/html"),
                _ => (404, "nope".to_string(), "text/plain"),
            };
            let header =
                tiny_http::Header::from_bytes(&b"Content-Type"[..], ctype.as_bytes()).unwrap();
            let resp = tiny_http::Response::from_string(body)
                .with_status_code(status)
                .with_header(header);
            let _ = req.respond(resp);
        }
    });
    port
}

#[tokio::test]
async fn robots_disallow_excludes_page() {
    let port = start_server_with_robots();
    let tmp = tempfile::tempdir().unwrap();
    let config = config_for(port, tmp.path().to_path_buf(), 500, false); // ignore_robots=false
    let fetcher = StaticFetcher::new(None).unwrap();

    let report = pipeline::run(&fetcher, &config).await.unwrap();

    // /docs/b 被 robots.txt 排除 → 只抓 /docs/a（A9 / C9）
    assert_eq!(report.ok, 1);
    assert!(tmp.path().join("docs/a.md").exists());
    assert!(!tmp.path().join("docs/b.md").exists());
}

fn sitemap_with_bad_link(port: u16) -> String {
    format!(
        "<?xml version=\"1.0\"?><urlset>\
         <url><loc>http://127.0.0.1:{port}/docs/a</loc></url>\
         <url><loc>http://127.0.0.1:{port}/docs/b</loc></url>\
         <url><loc>http://127.0.0.1:1/docs/c</loc></url>\
         </urlset>"
    )
}

fn start_server_with_bad_link() -> u16 {
    let server = tiny_http::Server::http("127.0.0.1:0").expect("bind");
    let port = server.server_addr().to_ip().expect("ip addr").port();
    std::thread::spawn(move || {
        for req in server.incoming_requests() {
            let path = req.url().split('?').next().unwrap_or("").to_string();
            let (status, body, ctype): (u16, String, &str) = match path.as_str() {
                "/sitemap.xml" => (200, sitemap_with_bad_link(port), "application/xml"),
                "/docs/a" => (200, doc_html("A", "/docs/b"), "text/html"),
                "/docs/b" => (200, doc_html("B", "/docs/a"), "text/html"),
                _ => (404, "nope".to_string(), "text/plain"),
            };
            let header =
                tiny_http::Header::from_bytes(&b"Content-Type"[..], ctype.as_bytes()).unwrap();
            let resp = tiny_http::Response::from_string(body)
                .with_status_code(status)
                .with_header(header);
            let _ = req.respond(resp);
        }
    });
    port
}

#[tokio::test]
async fn broken_link_counts_as_failed_and_triggers_failure() {
    let port = start_server_with_bad_link();
    let tmp = tempfile::tempdir().unwrap();
    let config = config_for(port, tmp.path().to_path_buf(), 500, false);
    let fetcher = StaticFetcher::new(None).unwrap();

    let report = pipeline::run(&fetcher, &config).await.unwrap();

    assert_eq!(report.failed, 1, "c should fail on port 1");
    assert!(report.failure_rate > 0.2);
    assert!(report.is_failure(0.2));
    assert_eq!(report.exit_code(0.2), 1);
    assert_eq!(report.ok, 2, "a and b should still succeed");
}
