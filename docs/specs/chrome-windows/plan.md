# Plan · Chrome 检测支持 Windows

- Feature ID: `chrome-windows`
- 上游: `spec.md` v1.1

## 实现方案

### 1. Windows 候选路径（可注入 env 的纯函数）

为同时满足「不写死盘符」（F2）与「跨平台可测」（F4），把路径构造抽成接收 env 查询闭包的纯函数 `windows_candidates`，`candidates()` 仅注入真实 `std::env::var`：

```rust
#[cfg(target_os = "windows")]
fn candidates() -> Vec<PathBuf> {
    windows_candidates(|k| std::env::var(k).ok())
}

#[cfg(any(target_os = "windows", test))]
fn windows_candidates(env: impl Fn(&str) -> Option<String>) -> Vec<PathBuf> {
    let env = |k: &str| env(k).filter(|s| !s.is_empty()); // 空串视为缺失（N1）
    let pf = env("ProgramFiles").unwrap_or_else(|| r"C:\Program Files".to_string());
    let pf86 = env("ProgramFiles(x86)").unwrap_or_else(|| r"C:\Program Files (x86)".to_string());
    let local = env("LOCALAPPDATA");
    let mut v = Vec::new();
    // 厂商优先级降序；每厂商内探测机器级(x64/x86) + per-user 三种安装范围
    for (dir, exe) in [
        (r"Google\Chrome\Application", "chrome.exe"),
        (r"Chromium\Application", "chrome.exe"),
        (r"Microsoft\Edge\Application", "msedge.exe"),
        (r"BraveSoftware\Brave-Browser\Application", "brave.exe"),
    ] {
        v.push(PathBuf::from(&pf).join(dir).join(exe));
        v.push(PathBuf::from(&pf86).join(dir).join(exe));
        if let Some(local) = &local {
            v.push(PathBuf::from(local).join(dir).join(exe)); // per-user（Brave 默认装这里，F1）
        }
    }
    v
}
```

要点：
- **F1**：per-user 分支覆盖全部四个厂商（Brave/Edge 默认即 per-user），不再只补 Chrome/Chromium。
- **F2**：机器级用 `%ProgramFiles%` / `%ProgramFiles(x86)%`，仅在变量缺失时回退默认 `C:\Program Files`。
- **N1**：env 查询统一经 `filter(空串视为缺失)`，被设为空串的变量同样回退默认 / 不产生 per-user 相对路径。
- **优先级**：厂商外层（Chrome→Chromium→Edge→Brave）、范围内层（x64→x86→user），保持 Chrome 系最高优先。

### 2. Linux 分支精确化 + 其它平台兜底（F3）

- `#[cfg(not(target_os = "macos"))]` → 精确 `#[cfg(target_os = "linux")]`（避免误覆盖 Windows）。
- 补 `#[cfg(not(any(macos, linux, windows)))]` 兜底分支返回空 `Vec`，确保任意 `target_os` 均可编译（宪法 §1）：

```rust
#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
fn candidates() -> Vec<PathBuf> {
    Vec::new()
}
```

### 3. 测试（F4）

- `windows_candidates(env)` 由跨平台单测注入伪 env 验证（AC1/AC2）：使用注入盘符、不含 `C:\Program Files`、包含 per-user Brave；env 全缺失时回退默认且非空。
- 空串 env 测试（N1）：三变量被设为空串时回退默认、不产生 per-user 路径。
- `candidates_non_empty` 仅在 macos/linux/windows 三平台断言非空（AC3），避免兜底平台误失败。
- `#[ignore]` 集成测试 `detect_chrome_on_real_host`（宪法 §7，opt-in 不阻塞默认套件）：本机真实检测留痕，证明 Success Metric #1。
- 现有 `choose()` 三 mode 单测保持（AC4）。

## 改动清单

| 文件 | 改动 |
| --- | --- |
| `src/fetcher/detect.rs` | `windows_candidates` 可注入 env 纯函数（含空串过滤）+ `#[cfg(windows)]` 注入真实 env + `#[cfg(linux)]` 限定 + 非三平台兜底分支 + 单测（注入 env / 空 env / 兜底 / `#[ignore]` 本机检测） |

无新增依赖。
