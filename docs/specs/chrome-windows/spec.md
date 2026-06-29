# Spec · Chrome 检测支持 Windows

- Feature ID: `chrome-windows`
- 状态: **v1.0（待确认）**
- 上游: `plan.md` M4.2、`constitution.md`

## Problem Statement

Chrome 自动检测当前仅覆盖 macOS 与 Linux 平台。Windows 用户使用 `--mode auto` 或 `--mode dynamic` 时，因 `detect.rs` 的 `candidates()` 无 Windows 分支而无法找到本机 Chrome，导致**动态引擎不可用**。

需补齐 Windows 平台的 Chrome / Chromium / Edge / Brave 安装路径扫描，保持与其它平台一致的 `--mode` 行为。

## Success Metrics

- Windows 上 `--mode auto` 能检测到已安装的 Chrome 系浏览器并启用动态引擎。
- `--mode dynamic` 无 Chrome 时报错退出。
- 各平台 `#[cfg(target_os)]` 编译正确，跨平台 CI 通过（现有 macOS Linux 行为不受影响）。

## Non-Goals

- 不通过注册表或 `where` 命令查找 Chrome（仅扫描固定路径 + 环境变量，与 macOS Linux 一致）。
- 不引入新依赖。

## Constraints

- 仅修改 `src/fetcher/detect.rs`。
- 不改变现有 macOS / Linux 逻辑。
- 跨平台编译（`#[cfg(target_os)]` 隔离）。
