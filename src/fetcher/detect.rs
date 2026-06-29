//! Chrome 检测与引擎模式决策（plan §6.1 / M4.2）。

use std::path::{Path, PathBuf};

use crate::cli::Mode;

/// 引擎决策结果。
#[derive(Debug)]
pub enum EngineChoice {
    Static,
    Dynamic(PathBuf),
}

/// 按优先级检测 Chrome 可执行文件；`explicit` 为 `--chrome-path`。
pub fn detect_chrome(explicit: Option<&Path>) -> Option<PathBuf> {
    if let Some(p) = explicit {
        if p.exists() {
            return Some(p.to_path_buf());
        }
    }
    candidates().into_iter().find(|p| p.exists())
}

#[cfg(target_os = "macos")]
fn candidates() -> Vec<PathBuf> {
    [
        "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
        "/Applications/Chromium.app/Contents/MacOS/Chromium",
        "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
        "/Applications/Brave Browser.app/Contents/MacOS/Brave Browser",
    ]
    .into_iter()
    .map(PathBuf::from)
    .collect()
}

#[cfg(not(target_os = "macos"))]
fn candidates() -> Vec<PathBuf> {
    [
        "/usr/bin/google-chrome",
        "/usr/bin/chromium",
        "/usr/bin/chromium-browser",
        "/usr/bin/microsoft-edge",
        "/snap/bin/chromium",
    ]
    .into_iter()
    .map(PathBuf::from)
    .collect()
}

/// 由 mode + 检测结果决策引擎（纯函数）。
pub fn choose(mode: Mode, detected: Option<PathBuf>) -> Result<EngineChoice, String> {
    match mode {
        Mode::Static => Ok(EngineChoice::Static),
        Mode::Dynamic => detected.map(EngineChoice::Dynamic).ok_or_else(|| {
            "--mode dynamic 但未检测到 Chrome；请安装或用 --chrome-path 指定".to_string()
        }),
        Mode::Auto => Ok(detected.map_or(EngineChoice::Static, EngineChoice::Dynamic)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_mode_always_static() {
        assert!(matches!(
            choose(Mode::Static, None),
            Ok(EngineChoice::Static)
        ));
        assert!(matches!(
            choose(Mode::Static, Some(PathBuf::from("/x"))),
            Ok(EngineChoice::Static)
        ));
    }

    #[test]
    fn dynamic_mode_needs_chrome() {
        assert!(matches!(
            choose(Mode::Dynamic, Some(PathBuf::from("/x"))),
            Ok(EngineChoice::Dynamic(_))
        ));
        assert!(choose(Mode::Dynamic, None).is_err());
    }

    #[test]
    fn auto_uses_chrome_or_falls_back() {
        assert!(matches!(
            choose(Mode::Auto, Some(PathBuf::from("/x"))),
            Ok(EngineChoice::Dynamic(_))
        ));
        assert!(matches!(choose(Mode::Auto, None), Ok(EngineChoice::Static)));
    }

    #[test]
    fn candidates_non_empty() {
        assert!(!candidates().is_empty());
    }
}
