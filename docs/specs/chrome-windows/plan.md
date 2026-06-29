# Plan · Chrome 检测支持 Windows

- Feature ID: `chrome-windows`
- 上游: `spec.md` v1.0

## 实现方案

### 1. 新增 Windows 路径分支

在 `src/fetcher/detect.rs` 的 `candidates()` 函数中新增 `#[cfg(target_os = "windows")]` 分支：

```rust
#[cfg(target_os = "windows")]
fn candidates() -> Vec<PathBuf> {
    let mut v = Vec::new();
    // 固定安装路径（优先级降序）
    for base in [
        r"C:\Program Files\Google\Chrome\Application\chrome.exe",
        r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
        r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
        r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
        r"C:\Program Files\BraveSoftware\Brave-Browser\Application\brave.exe",
    ] {
        v.push(PathBuf::from(base));
    }
    // %LOCALAPPDATA% 下的用户级安装路径
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        v.push(PathBuf::from(local).join(r"Google\Chrome\Application\chrome.exe"));
        v.push(PathBuf::from(local).join(r"Chromium\Application\chrome.exe"));
    }
    v
}
```

### 2. Linux 分支精确化

当前 `#[cfg(not(target_os = "macos"))]` 会误覆盖 Windows，改为精确的 `#[cfg(target_os = "linux")]`。

### 3. 测试

`candidates_non_empty` 测试（现有）在各平台验证 candidates 列表非空。

## 改动清单

| 文件 | 改动 |
| --- | --- |
| `src/fetcher/detect.rs` | 加 `#[cfg(windows)]` 分支 + `#[cfg(linux)]` 限定 + 路径列表 |

无新增依赖。
