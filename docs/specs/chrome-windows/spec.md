# Spec · Chrome 检测支持 Windows

- Feature ID: `chrome-windows`
- 状态: **v1.1（审查整改：F1 Brave per-user 漏检 / F2 硬编码盘符 / F3 跨平台编译回归 / F4 验收可测性）**
- 上游: `plan.md` M4.2、`constitution.md`

## Problem Statement

Chrome 自动检测当前仅覆盖 macOS 与 Linux 平台。Windows 用户使用 `--mode auto` 或 `--mode dynamic` 时，因 `detect.rs` 的 `candidates()` 无 Windows 分支而无法找到本机 Chrome，导致**动态引擎不可用**。

需补齐 Windows 平台的 Chrome / Chromium / Edge / Brave 安装路径扫描，保持与其它平台一致的 `--mode` 行为。

## User Stories

- 作为 **Windows + 系统级安装 Chrome/Edge** 的用户，运行 `--mode auto` 时能自动启用动态引擎，无需 `--chrome-path`。
- 作为 **Windows + per-user 安装 Brave/Chrome/Edge**（默认装到 `%LOCALAPPDATA%`）的用户，同样能被自动检测到（F1）。
- 作为 **系统盘非 C: 或 Program Files 被重定向** 的用户，检测仍按真实安装位置进行，不因写死 `C:\` 而失效（F2）。

## Success Metrics

- Windows 上 `--mode auto` 能检测到已安装的 Chrome 系浏览器（Chrome / Chromium / Edge / Brave）并启用动态引擎，**覆盖机器级与 per-user 两种安装范围**。
- `--mode dynamic` 无 Chrome 时报错退出。
- 候选路径基于 `%ProgramFiles%` / `%ProgramFiles(x86)%` / `%LOCALAPPDATA%` 环境变量推导（环境变量缺失时回退到默认 `C:\Program Files`），**不写死系统盘符**。
- 所有受支持及未来新增 `target_os` 均能编译（含非 macos/linux/windows 平台的兜底分支），跨平台 CI 通过（现有 macOS / Linux 行为不受影响）。

## Acceptance Criteria

- AC1：注入伪造环境变量（如 `ProgramFiles=D:\PF`、`LOCALAPPDATA=D:\...\AppData\Local`）时，生成的候选列表使用注入盘符、**不含 `C:\Program Files`**，且包含 per-user Brave 路径 —— 由跨平台单测验证（不依赖真实 Windows 安装）。
- AC2：环境变量全缺失时，候选列表非空且回退到默认 `C:\Program Files\...`。
- AC3：在 macos / linux / windows 上 `candidates()` 非空；在其它 `target_os` 上 crate 仍能编译。
- AC4：现有 `choose()` 三种 mode 行为单测保持绿。

## Non-Goals

- 不通过注册表或 `where` 命令查找 Chrome（仅扫描固定路径 + 环境变量，与 macOS Linux 一致）。
- 不引入新依赖。

## Constraints

- 改动局限于平台检测模块（`fetcher` 内的 `detect`），不影响其它业务模块（宪法 §3）。
- 不改变现有 macOS / Linux 的检测语义。
- 跨平台编译，平台差异经 `#[cfg(target_os)]` 隔离；**不写死平台盘符**（宪法 §1）。
