//! 图片下载与本地化（plan §6.6 / M3.1+M3.2 / N3：仅图片）。
//!
//! 文件名 = `sha256(src)[..16] + ext`（稳定去重）；扩展名优先取 URL，缺失用 `Content-Type` 兜底。
//! 本模块只下载并返回本地文件名；HTML 改写由 `rewrite` 执行（T-5）。相对深度用 `urlx::relative_path`。

use std::path::Path;

use sha2::{Digest, Sha256};
use url::Url;

use crate::error::{Error, Result};

/// 下载单张图片到 `assets_dir`，返回本地文件名（已存在则跳过下载，去重）。
pub async fn download_image(
    client: &reqwest::Client,
    src: &str,
    assets_dir: &Path,
) -> Result<String> {
    let url = Url::parse(src).map_err(|e| Error::Extract(format!("img url {src}: {e}")))?;
    let stem = sha16(src);

    // URL 含扩展名时，下载前即可去重
    if let Some(ext) = url_ext(&url) {
        let name = format!("{stem}.{ext}");
        if assets_dir.join(&name).exists() {
            return Ok(name);
        }
    }

    let resp = client
        .get(url.clone())
        .send()
        .await
        .map_err(|e| Error::Extract(format!("img {src}: {e}")))?;
    if !resp.status().is_success() {
        return Err(Error::Extract(format!(
            "img {src}: status {}",
            resp.status()
        )));
    }

    let ext = url_ext(&url)
        .or_else(|| ext_from_content_type(resp.headers().get(reqwest::header::CONTENT_TYPE)))
        .unwrap_or_else(|| "bin".to_string());
    let name = format!("{stem}.{ext}");
    let path = assets_dir.join(&name);
    if !path.exists() {
        let bytes = resp
            .bytes()
            .await
            .map_err(|e| Error::Extract(format!("img {src}: body {e}")))?;
        std::fs::create_dir_all(assets_dir)?;
        std::fs::write(&path, &bytes)?;
    }
    Ok(name)
}

/// `sha256(s)` 前 8 字节 → 16 位十六进制。
fn sha16(s: &str) -> String {
    Sha256::digest(s.as_bytes())
        .iter()
        .take(8)
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// URL 路径末段扩展名（仅接受短的字母数字，视为图片扩展名）。
fn url_ext(url: &Url) -> Option<String> {
    let last = url.path_segments()?.next_back()?;
    let (_, ext) = last.rsplit_once('.')?;
    if !ext.is_empty() && ext.len() <= 5 && ext.chars().all(|c| c.is_ascii_alphanumeric()) {
        Some(ext.to_ascii_lowercase())
    } else {
        None
    }
}

/// 由响应 `Content-Type` 推断扩展名。
fn ext_from_content_type(ct: Option<&reqwest::header::HeaderValue>) -> Option<String> {
    let raw = ct?.to_str().ok()?;
    let mime = raw.split(';').next()?.trim();
    let exts = mime_guess::get_mime_extensions_str(mime)?;
    exts.first().map(|e| (*e).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha16_is_deterministic_16_hex() {
        let a = sha16("https://x.com/a.png");
        assert_eq!(a.len(), 16);
        assert_eq!(a, sha16("https://x.com/a.png"));
        assert_ne!(a, sha16("https://x.com/b.png"));
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn url_ext_extracts_image_extension() {
        let p = |s: &str| url_ext(&Url::parse(s).unwrap());
        assert_eq!(p("https://x.com/a/b.png"), Some("png".to_string()));
        assert_eq!(p("https://x.com/a/B.SVG"), Some("svg".to_string()));
        assert_eq!(p("https://x.com/img?id=1"), None);
        assert_eq!(p("https://x.com/noext"), None);
    }

    #[test]
    fn ext_from_mime_works() {
        use reqwest::header::HeaderValue;
        let hv = HeaderValue::from_static("image/png; charset=utf-8");
        assert_eq!(ext_from_content_type(Some(&hv)), Some("png".to_string()));
    }
}
