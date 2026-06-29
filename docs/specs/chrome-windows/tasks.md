# Tasks · Chrome 检测支持 Windows

- Feature ID: `chrome-windows`
- 上游: `spec.md` v1.0、`plan.md` v1.0

| ID | 任务 | 依赖 | 输出 | 完成标准 | 状态 |
| --- | --- | --- | --- | --- | --- |
| CW-1 | 在 `detect.rs` 加 `#[cfg(windows)]` candidates + `#[cfg(linux)]` 限定 | — | `src/fetcher/detect.rs` | 编译通过；`candidates_non_empty` 测试绿 | ✅ |

> 仅 1 处文件修改，单任务即可覆盖。
