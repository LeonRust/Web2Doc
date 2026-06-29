# Tasks · Chrome 检测支持 Windows

- Feature ID: `chrome-windows`
- 上游: `spec.md` v1.1、`plan.md` v1.1

| ID | 任务 | 依赖 | 输出 | 完成标准 | 状态 |
| --- | --- | --- | --- | --- | --- |
| CW-1 | 在 `detect.rs` 加 `#[cfg(windows)]` candidates + `#[cfg(linux)]` 限定 | — | `src/fetcher/detect.rs` | 编译通过；`candidates_non_empty` 测试绿 | ✅ |
| CW-2 | 重构为可注入 env 的 `windows_candidates`：`%ProgramFiles%`/`%ProgramFiles(x86)%`/`%LOCALAPPDATA%`，per-user 覆盖全部四厂商（含 Brave） | CW-1 | `src/fetcher/detect.rs` | 注入伪 env 时不含 `C:\Program Files`、含 per-user Brave（AC1）；env 缺失回退默认且非空（AC2） | ✅ |
| CW-3 | 补非 macos/linux/windows 兜底 `candidates()` 分支 | CW-1 | `src/fetcher/detect.rs` | 任意 `target_os` 可编译；`candidates_non_empty` 限三平台断言（AC3） | ✅ |
| CW-4 | 新增跨平台单测覆盖 AC1/AC2，保留 `choose()` 三 mode 测试 | CW-2, CW-3 | `src/fetcher/detect.rs` | `cargo fmt --check && cargo clippy -D warnings && cargo test` 全绿（AC4） | ✅ |
| CW-5 | 二轮审查整改：空 env 视为缺失回退默认（N1）；新增 `#[ignore]` 本机检测集成测试 | CW-2 | `src/fetcher/detect.rs` | 空 env 测试绿；`detect_chrome_on_real_host` 经 `cargo test -- --ignored` 在本机绿 | ✅ |

> CW-1 为初版（已暴露 F1/F2/F3/F4）；CW-2~CW-4 为首轮整改；CW-5 为二轮整改（N1/Validate 留痕）。
>
> **验证留痕（Validate）**：见本文件末「验证记录」。

## 验证记录（Validate）

环境：Windows（win32），`cargo` 工具链。

| 门禁 | 命令 | 结果 |
| --- | --- | --- |
| 格式 | `cargo fmt -- --check` | 通过（exit 0） |
| 静态检查 | `cargo clippy --all-targets -- -D warnings` | 通过（exit 0） |
| 全量测试 | `cargo test` | **80 passed / 0 failed / 2 ignored**（含 5 集成测试）（exit 0） |
| 真实检测（Metric #1） | `cargo test detect_chrome_on_real_host -- --ignored --nocapture` | 通过；本机命中 `detect_chrome(None) = Some("C:\Program Files\Google\Chrome\Application\chrome.exe")` |

- Success Metric #1（自动启用动态引擎）已由本机真实检测留痕证明。
- Metric #2（`--mode dynamic` 无 Chrome 报错退出）由 `dynamic_mode_needs_chrome` 单测在决策层证明（`choose(Dynamic, None)` 返回 Err）。
- AC3 兜底分支跨目标编译为静态核验（四 `#[cfg]` 臂互斥且穷尽）；未做跨目标 `cargo check`（依赖树含 C/原生组件，wasm 等目标不可构建，非本改动问题）。
