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

#[cfg(target_os = "linux")]
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

#[cfg(target_os = "windows")]
fn candidates() -> Vec<PathBuf> {
    windows_candidates(|k| std::env::var(k).ok())
}

#[cfg(any(target_os = "windows", test))]
fn windows_candidates(env: impl Fn(&str) -> Option<String>) -> Vec<PathBuf> {
    let env = |k: &str| env(k).filter(|s| !s.is_empty());
    let pf = env("ProgramFiles").unwrap_or_else(|| r"C:\Program Files".to_string());
    let pf86 = env("ProgramFiles(x86)").unwrap_or_else(|| r"C:\Program Files (x86)".to_string());
    let local = env("LOCALAPPDATA");
    let mut v = Vec::new();
    for (dir, exe) in [
        (r"Google\Chrome\Application", "chrome.exe"),
        (r"Chromium\Application", "chrome.exe"),
        (r"Microsoft\Edge\Application", "msedge.exe"),
        (r"BraveSoftware\Brave-Browser\Application", "brave.exe"),
    ] {
        v.push(PathBuf::from(&pf).join(dir).join(exe));
        v.push(PathBuf::from(&pf86).join(dir).join(exe));
        if let Some(local) = &local {
            v.push(PathBuf::from(local).join(dir).join(exe));
        }
    }
    v
}

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
fn candidates() -> Vec<PathBuf> {
    Vec::new()
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
    #[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
    fn candidates_non_empty() {
        assert!(!candidates().is_empty());
    }

    #[test]
    fn windows_candidates_use_injected_env_and_cover_per_user_brave() {
        let env = |k: &str| match k {
            "ProgramFiles" => Some(r"D:\PF".to_string()),
            "ProgramFiles(x86)" => Some(r"D:\PF86".to_string()),
            "LOCALAPPDATA" => Some(r"D:\Users\me\AppData\Local".to_string()),
            _ => None,
        };
        let paths: Vec<String> = windows_candidates(env)
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect();
        assert!(paths.iter().any(|p| p.contains(r"D:\PF")));
        assert!(paths.iter().any(|p| p.contains(r"D:\PF86")));
        assert!(paths.iter().all(|p| !p.contains(r"C:\Program Files")));
        assert!(paths.iter().any(|p| {
            p.contains(r"AppData\Local")
                && p.contains(r"BraveSoftware\Brave-Browser\Application")
                && p.contains("brave.exe")
        }));
    }

    #[test]
    fn windows_candidates_fall_back_to_default_program_files() {
        let paths: Vec<String> = windows_candidates(|_| None)
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect();
        assert!(!paths.is_empty());
        assert!(paths
            .iter()
            .any(|p| p.contains(r"C:\Program Files") && p.contains("chrome.exe")));
        assert!(paths.iter().all(|p| !p.contains("AppData")));
    }

    #[test]
    fn windows_candidates_treat_empty_env_as_absent() {
        let paths: Vec<String> = windows_candidates(|k| match k {
            "ProgramFiles" | "ProgramFiles(x86)" | "LOCALAPPDATA" => Some(String::new()),
            _ => None,
        })
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
        assert!(paths.iter().any(|p| p.contains(r"C:\Program Files")));
        assert!(paths.iter().all(|p| !p.contains("AppData")));
    }

    #[test]
    #[ignore = "环境相关：需本机安装 Chrome 系浏览器；手动 `cargo test -- --ignored` 验证 Success Metric #1"]
    fn detect_chrome_on_real_host() {
        let found = detect_chrome(None);
        eprintln!("detect_chrome(None) = {found:?}");
        assert!(found.is_some(), "本机未检测到 Chrome 系浏览器");
    }
}
