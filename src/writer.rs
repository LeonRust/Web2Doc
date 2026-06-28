//! 产物落盘（plan §6.8 / S5/S6）：核心 manifest 类型、原子写、镜像 `.md`、总索引。
//!
//! URL→rel_path 映射算法在 `urlx`（C-1）；本模块负责持久化与目录写出。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::rules::RuleSet;

const MANIFEST_FILE: &str = "manifest.json";
const INDEX_FILE: &str = "index.md";

/// 页面状态机（plan §5 / N-1 / T-1）。续传按 `Written` 判定。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PageStatus {
    Pending,
    Fetched,
    Written,
    Failed,
    Excluded,
}

/// 单页记录（落入 manifest，plan §5）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageRecord {
    pub url: String,
    pub rel_path: String,
    pub status: PageStatus,
    pub cache: Option<String>,
    pub assets: Vec<String>,
    pub error: Option<String>,
}

/// 抓取进度清单（`<out>/manifest.json`，原子写，支持续传）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub root_url: String,
    pub prefix: String,
    pub rules: RuleSet,
    pub baseline_total: usize,
    pub truncated: bool,
    pub nav_order: Vec<String>,
    /// key = 规范化去重键（`urlx::dedup_key`）。
    pub pages: BTreeMap<String, PageRecord>,
    /// 规范化绝对 src_url → 本地相对路径（去重，M3 使用）。
    pub assets_seen: BTreeMap<String, String>,
}

impl Manifest {
    /// 读取既有 manifest（不存在 / 损坏 → `None`，触发全新抓取）。
    pub fn load(out_dir: &Path) -> Option<Manifest> {
        let data = std::fs::read_to_string(out_dir.join(MANIFEST_FILE)).ok()?;
        serde_json::from_str(&data).ok()
    }

    /// 原子写：写 `manifest.json.tmp` 后 `rename`，避免崩溃产生损坏文件（I1）。
    pub fn save_atomic(&self, out_dir: &Path) -> Result<()> {
        std::fs::create_dir_all(out_dir)?;
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| Error::Extract(format!("manifest serialize: {e}")))?;
        let tmp = out_dir.join("manifest.json.tmp");
        std::fs::write(&tmp, json)?;
        std::fs::rename(&tmp, out_dir.join(MANIFEST_FILE))?;
        Ok(())
    }
}

/// 安全拼接输出路径：拒绝绝对路径与 `..`（越界，constitution §5）。
fn safe_join(out_dir: &Path, rel: &str) -> Result<PathBuf> {
    if rel.is_empty() || rel.starts_with('/') || rel.split('/').any(|s| s == ".." || s == ".") {
        return Err(Error::PathEscape(rel.to_string()));
    }
    Ok(out_dir.join(rel))
}

/// 写单页 Markdown（按 rel_path 镜像目录，自动建父目录）。
pub fn write_markdown(out_dir: &Path, rel_path: &str, content_md: &str) -> Result<()> {
    let full = safe_join(out_dir, rel_path)?;
    if let Some(parent) = full.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&full, content_md)?;
    Ok(())
}

/// 写总索引 `index.md`：按 `nav_order` 列出 `Written` 页面，回退键字典序。
pub fn write_index(out_dir: &Path, manifest: &Manifest) -> Result<()> {
    let order: Vec<&String> = if manifest.nav_order.is_empty() {
        manifest.pages.keys().collect()
    } else {
        manifest.nav_order.iter().collect()
    };

    let mut md = String::from("# Index\n\n");
    for key in order {
        if let Some(rec) = manifest.pages.get(key) {
            if rec.status == PageStatus::Written {
                md.push_str(&format!("- [{0}]({0})\n", rec.rel_path));
            }
        }
    }
    std::fs::create_dir_all(out_dir)?;
    std::fs::write(out_dir.join(INDEX_FILE), md)?;
    Ok(())
}

/// 写全文合并产物 `_bundle.md`（`--bundle`，C8）：按 nav_order 拼接 Written 页 + 来源注释；
/// 图片路径按根层级重算（N-7：`(../)*assets/` → `assets/`，防深度错位死链）。
pub fn write_bundle(out_dir: &Path, manifest: &Manifest) -> Result<()> {
    let order: Vec<&String> = if manifest.nav_order.is_empty() {
        manifest.pages.keys().collect()
    } else {
        manifest.nav_order.iter().collect()
    };

    let mut bundle = String::from("# Bundle\n\n");
    for key in order {
        if let Some(rec) = manifest.pages.get(key) {
            if rec.status == PageStatus::Written {
                if let Ok(content) = std::fs::read_to_string(out_dir.join(&rec.rel_path)) {
                    bundle.push_str(&format!("\n<!-- source: {} -->\n\n", rec.url));
                    bundle.push_str(&fix_bundle_asset_paths(&content));
                    bundle.push_str("\n\n---\n\n");
                }
            }
        }
    }
    std::fs::create_dir_all(out_dir)?;
    std::fs::write(out_dir.join("_bundle.md"), bundle)?;
    Ok(())
}

/// bundle 位于输出根，故把任意深度的 `../assets/` 前缀归一为 `assets/`。
fn fix_bundle_asset_paths(md: &str) -> String {
    let mut s = md.to_string();
    while s.contains("../assets/") {
        s = s.replace("../assets/", "assets/");
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(url: &str, rel: &str, status: PageStatus) -> PageRecord {
        PageRecord {
            url: url.to_string(),
            rel_path: rel.to_string(),
            status,
            cache: None,
            assets: vec![],
            error: None,
        }
    }

    fn sample() -> Manifest {
        let mut pages = BTreeMap::new();
        pages.insert(
            "https://x.com/docs/a".to_string(),
            rec("https://x.com/docs/a", "docs/a.md", PageStatus::Written),
        );
        pages.insert(
            "https://x.com/docs/b".to_string(),
            rec("https://x.com/docs/b", "docs/b.md", PageStatus::Excluded),
        );
        Manifest {
            root_url: "https://x.com/docs/".to_string(),
            prefix: "/docs/".to_string(),
            rules: RuleSet::fallback(),
            baseline_total: 2,
            truncated: false,
            nav_order: vec![
                "https://x.com/docs/a".to_string(),
                "https://x.com/docs/b".to_string(),
            ],
            pages,
            assets_seen: BTreeMap::new(),
        }
    }

    #[test]
    fn safe_join_rejects_escape() {
        let out = Path::new("/tmp/out");
        assert!(safe_join(out, "../etc/passwd").is_err());
        assert!(safe_join(out, "/abs").is_err());
        assert!(safe_join(out, "a/../b").is_err());
        assert!(safe_join(out, "docs/a.md").is_ok());
    }

    #[test]
    fn writes_markdown_mirroring_path() {
        let tmp = tempfile::tempdir().unwrap();
        write_markdown(tmp.path(), "docs/guide/a.md", "# A\n").unwrap();
        let got = std::fs::read_to_string(tmp.path().join("docs/guide/a.md")).unwrap();
        assert_eq!(got, "# A\n");
    }

    #[test]
    fn manifest_atomic_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let m = sample();
        m.save_atomic(tmp.path()).unwrap();
        assert!(tmp.path().join("manifest.json").exists());
        assert!(!tmp.path().join("manifest.json.tmp").exists());
        let loaded = Manifest::load(tmp.path()).unwrap();
        assert_eq!(loaded.baseline_total, 2);
        assert_eq!(loaded.pages.len(), 2);
        assert_eq!(loaded.pages["https://x.com/docs/a"].rel_path, "docs/a.md");
    }

    #[test]
    fn index_lists_only_written_in_nav_order() {
        let tmp = tempfile::tempdir().unwrap();
        write_index(tmp.path(), &sample()).unwrap();
        let idx = std::fs::read_to_string(tmp.path().join("index.md")).unwrap();
        assert!(idx.contains("[docs/a.md](docs/a.md)"));
        assert!(!idx.contains("docs/b.md")); // Excluded 不列入
    }

    #[test]
    fn load_missing_is_none() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(Manifest::load(tmp.path()).is_none());
    }

    #[test]
    fn bundle_normalizes_asset_paths() {
        assert_eq!(
            fix_bundle_asset_paths("![](../assets/x.png)"),
            "![](assets/x.png)"
        );
        assert_eq!(
            fix_bundle_asset_paths("![](../../assets/y.png)"),
            "![](assets/y.png)"
        );
        assert_eq!(
            fix_bundle_asset_paths("![](assets/z.png)"),
            "![](assets/z.png)"
        );
    }

    #[test]
    fn write_bundle_concatenates_written_pages() {
        let tmp = tempfile::tempdir().unwrap();
        let m = sample();
        // 写出 Written 页 a 的 md（含深层图片路径）
        write_markdown(tmp.path(), "docs/a.md", "# A\n\n![](../../assets/i.png)\n").unwrap();
        write_bundle(tmp.path(), &m).unwrap();
        let b = std::fs::read_to_string(tmp.path().join("_bundle.md")).unwrap();
        assert!(b.contains("# A"));
        assert!(b.contains("](assets/i.png)"), "asset path normalized: {b}");
        assert!(!b.contains("docs/b.md")); // Excluded 不入 bundle
    }
}
