# Web2Doc

将在线文档站点抓取为本地结构化 Markdown/HTML，供 AI Coding 工具作为上下文约束使用。

[![Rust](https://img.shields.io/badge/rust-1.85+-orange)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue)](./LICENSE)
[![Test](https://img.shields.io/badge/test-80%20unit%20%2B%205%20integration-green)]()

---

## 目录

- [系统要求](#系统要求)
- [安装](#安装)
- [快速开始](#快速开始)
- [引擎架构](#引擎架构)
- [CLI 参考](#cli-参考)
- [环境变量](#环境变量)
- [用法指南](#用法指南)
- [输出格式](#输出格式)
- [产物结构](#产物结构)
- [LLM 规则分析](#llm-规则分析)
- [覆盖率与失败口径](#覆盖率与失败口径)
- [真实站点验证](#真实站点验证)
- [作为 AI Coding Skill 使用](#作为-ai-coding-skill-使用)
- [开发](#开发)
- [故障排查](#故障排查)

---

## 系统要求

| 组件 | 最低版本 | 说明 |
| --- | --- | --- |
| Rust | **1.85+** (MSRV) | `rustup` 安装 |
| Chrome / Chromium | 任意近期版本 | **可选**，仅 `--mode dynamic` 需要 |

### Chrome 支持矩阵

| 平台 | 自动检测路径（优先级降序） |
| --- | --- |
| **macOS** | `Google Chrome.app` → `Chromium.app` → `Edge.app` → `Brave Browser.app` |
| **Linux** | `/usr/bin/google-chrome` → `/usr/bin/chromium` → `/usr/bin/chromium-browser` → `/usr/bin/microsoft-edge` → `/snap/bin/chromium` |
| **Windows** | `%ProgramFiles%\Google\Chrome` → `%ProgramFiles(x86)%\Google\Chrome` → `%ProgramFiles%\Microsoft\Edge` → `%ProgramFiles(x86)%\Microsoft\Edge` → `%ProgramFiles%\BraveSoftware\Brave-Browser` → `%LOCALAPPDATA%\Google\Chrome` → `%LOCALAPPDATA%\Chromium` |

> 所有平台均支持通过 `--chrome-path` 手动指定 Chrome 可执行文件路径。

---

## 安装

### 方式一：一键安装脚本（预编译，无需 Rust，推荐）

Linux / macOS：

```bash
curl -fsSL https://raw.githubusercontent.com/LeonRust/Web2Doc/main/scripts/install.sh | bash
```

Windows（PowerShell）：

```powershell
irm https://raw.githubusercontent.com/LeonRust/Web2Doc/main/scripts/install.ps1 | iex
```

自动识别平台，从 GitHub Releases 下载对应预编译二进制（带 SHA256 校验）。安装目录：Linux/macOS 为 `~/.local/bin`（可 `... | bash -s -- ~/bin` 指定）；Windows 为 `%LOCALAPPDATA%\web2doc\bin` 并写入用户 PATH。

### 方式二：手动下载预编译二进制

从 [GitHub Releases](https://github.com/LeonRust/Web2Doc/releases) 下载对应平台的压缩包，解压后把 `web2doc` 放到 `PATH`：

| 平台 | 附件 |
| --- | --- |
| Linux x86_64（glibc） | `web2doc-x86_64-unknown-linux-gnu.tar.gz` |
| Linux ARM64（glibc） | `web2doc-aarch64-unknown-linux-gnu.tar.gz` |
| Linux x86_64（musl，静态） | `web2doc-x86_64-unknown-linux-musl.tar.gz` |
| Linux ARM64（musl，静态） | `web2doc-aarch64-unknown-linux-musl.tar.gz` |
| macOS Intel | `web2doc-x86_64-apple-darwin.tar.gz` |
| macOS Apple Silicon | `web2doc-aarch64-apple-darwin.tar.gz` |
| Windows x86_64 | `web2doc-x86_64-pc-windows-msvc.zip` |
| Windows ARM64 | `web2doc-aarch64-pc-windows-msvc.zip` |

每个附件附带 `.sha256` 校验文件。版本由打 `vX.Y.Z` tag 自动构建并发布（见[开发 › 发布](#开发)）。

### 方式三：从源码构建（需 Rust 1.85+）

```bash
git clone https://github.com/LeonRust/Web2Doc
cd web2doc
cargo build --release           # 二进制位于 target/release/web2doc
cargo install --path .          # 可选：安装到 $PATH
```

---

## 快速开始

```bash
# 抓取任意文档站（默认静态引擎，无需 Chrome）
web2doc https://api-docs.deepseek.com/zh-cn/ --max-pages 50 -o ./docs

# SPA 站点需要开启动态引擎
web2doc https://spa-docs.example.com --mode dynamic

# 启用 LLM 分析 + 合并产物（投喂 AI）
export LLM_API_KEY=sk-...
web2doc https://docs.example.com --bundle -o ./ai-context
```

---

## 引擎架构

Web2Doc 采用**双引擎**设计，根据站点类型自动/手动选择：

```
                 ┌───────────────────────┐
                 │   start URL           │
                 └─────────┬─────────────┘
                           │
                   ┌───────▼─────────┐
                   │   --mode ?      │
                   │ auto/static/    │
                   │ dynamic         │
                   └───┬─────┬───────┘
           ┌───────────┘     └───────────┐
           ▼                             ▼
  ┌─────────────────┐          ┌───────────────────┐
  │  Static Engine  │          │  Dynamic Engine   │
  │  (reqwest)      │          │  (chromiumoxide)  │
  │                 │          │                   │
  │  SSR / SSG 站点  │          │  SPA / CSR 站点   │
  │  · Docusaurus   │          │  · 客户端渲染       │
  │  · VitePress    │          │  · 需 Chrome      │
  │  · mdBook       │          │  · headless 模式   │
  │  · GitBook      │          │                   │
  └────────┬────────┘          └─────────┬─────────┘
           │                             │
           └───────────┬─────────────────┘
                       ▼
             ┌───────────────────┐
             │  Pipeline         │
             │  · discover       │
             │  · readability    │
             │  · rewrite        │
             │  · convert/write  │
             └────────┬──────────┘
                      ▼
              ┌───────────────┐
              │  ./out/       │
              │  *.md / *.html│
              └───────────────┘
```

### 引擎模式

| 模式 | CLI | 行为 |
| --- | --- | --- |
| **自动**（默认） | `--mode auto` | 检测 Chrome → 有则 Dynamic，无则 Static + 告警 |
| **静态** | `--mode static` | 纯 HTTP，不启动浏览器 |
| **动态** | `--mode dynamic` | 强制 Chrome 渲染，无 Chrome 则报错退出 |

### 静态 vs 动态

| 维度 | Static | Dynamic |
| --- | --- | --- |
| 依赖 | 无外部依赖 | 需 Chrome / Chromium / Edge / Brave |
| 适用站点 | SSR / SSG | SPA / CSR |
| 速度 | 快 | 较慢（每页启 tab + 渲染） |
| 资源占用 | 低 | 中等（浏览器进程） |
| 默认模式 | `auto` 无 Chrome 时使用 | `auto` 有 Chrome 时使用 |

---

## CLI 参考

```
web2doc <URL> [选项]
```

### 必需参数

| 参数 | 说明 |
| --- | --- |
| `<URL>` | 文档站首页地址 |

### 页面发现

| 选项 | 类型 | 默认值 | 说明 |
| --- | --- | --- | --- |
| `--prefix <PATH>` | string | URL 路径目录 | 限定抓取前缀，例 `--prefix /docs/api/` |
| `--include-prefix <PATH>` | string（可多次） | — | 追加允许前缀 |
| `--max-pages <N>` | integer | 500 | 最大页数上限；超过则标记 Partial（非失败） |

### 抓取控制

| 选项 | 类型 | 默认值 | 说明 |
| --- | --- | --- | --- |
| `--mode` | `auto` \| `static` \| `dynamic` | `auto` | 引擎选择 |
| `--concurrency <N>` | integer | 4 | 并发请求数 |
| `--delay-ms <MS>` | integer | 500 | 请求间隔（毫秒），礼貌限速 |
| `--chrome-path <PATH>` | path | 自动检测 | Chrome 可执行文件路径（三平台兼容） |
| `--proxy <URL>` | string | — | 出站代理（http/https/socks5），覆盖静态+动态引擎 |
| `--no-proxy <LIST>` | string | — | 代理绕过列表（逗号分隔，如 `localhost,127.0.0.1`） |
| `--fresh` | flag | false | 忽略已有进度，从头抓取 |
| `--ignore-robots` | flag | false | 忽略 `robots.txt` |

### 输出

| 选项 | 类型 | 默认值 | 说明 |
| --- | --- | --- | --- |
| `--out <DIR>` | path | `./web2doc-out` | 产物输出目录 |
| `--format` | `md` \| `html` | `md` | 输出格式 |
| `--bundle` | flag | false | 额外输出全文合并文件 `_bundle.md` |
| `--max-failure-rate <F>` | float | 0.20 | 失败率阈值（超过判整次失败） |

### LLM 规则分析

LLM 三项配置（端点 / 模型 / 密钥）支持多种来源，优先级 **CLI > 环境变量 / .env > 配置文件 > 默认**（API Key 仅 .env / 环境变量 / 配置文件，不接受命令行明文）。

| 选项 | 类型 | 默认值 | 说明 |
| --- | --- | --- | --- |
| `--base-url <URL>` | string | `https://api.deepseek.com` | LLM 端点（OpenAI 兼容） |
| `--model <NAME>` | string | `deepseek-v4-flash` | LLM 模型名 |

### 日志

| 选项 | 说明 |
| --- | --- |
| `-v` | DEBUG 级别（显示每页抓取耗时、阶段进度） |
| `-vv` | TRACE 级别（含依赖库详细日志） |

---

## 环境变量 / .env

LLM 与代理配置均可通过 shell 或项目根目录 `.env` 文件设置（`.env` 不覆盖已存在的环境变量）：

| 变量 | 说明 |
| --- | --- |
| `LLM_BASE_URL` | LLM 端点（优先级高于配置文件，低于 `--base-url`） |
| `LLM_MODEL` | LLM 模型名（优先级高于配置文件，低于 `--model`） |
| `LLM_API_KEY` | LLM API Key（优先级高于配置文件）；设置后才启用 LLM 规则分析 |
| `ALL_PROXY` / `HTTPS_PROXY` / `HTTP_PROXY` | 出站代理（标准变量名；解析顺序 ALL→HTTPS→HTTP，低于 `--proxy`） |
| `NO_PROXY` | 代理绕过列表（低于 `--no-proxy`） |
| `RUST_LOG` | 覆盖默认日志级别（例 `RUST_LOG=web2doc=debug`） |

`.env` 示例：
```bash
LLM_BASE_URL=https://api.deepseek.com
LLM_MODEL=deepseek-v4-flash
LLM_API_KEY=sk-...
HTTPS_PROXY=http://127.0.0.1:7890
NO_PROXY=localhost,127.0.0.1
```

## 配置文件

可在用户配置目录放置 `web2doc/config.toml` 持久化 LLM / 代理设置（优先级低于 CLI 与环境变量）：

| 平台 | 路径 |
| --- | --- |
| **macOS / Linux** | `~/.config/web2doc/config.toml`（遵循 `$XDG_CONFIG_HOME`） |
| **Windows** | `%APPDATA%\web2doc\config.toml` |

```toml
[llm]
base_url = "https://api.openai.com/v1"
model = "gpt-4o"
api_key = "sk-..."

[proxy]
url = "http://127.0.0.1:7890"
no_proxy = "localhost,127.0.0.1,*.internal"
```

所有字段均可省略；缺失项回退到默认值（`base_url=https://api.deepseek.com`、`model=deepseek-v4-flash`；代理默认直连）。文件不存在或解析失败均不影响抓取（仅 LLM 规则分析降级为内置默认规则）。

> **代理说明**：`--proxy` 同时作用于静态引擎（reqwest）与动态引擎（Chrome），覆盖页面抓取 / 图片下载 / robots / LLM 全部出站流量。静态引擎支持带认证代理（`http://user:pass@host:port`）与 SOCKS5；**动态引擎（Chrome）暂不支持带认证的代理**，如需认证代理请改用 `--mode static`。

---

## 用法指南

### 场景 1：SSR/SSG 文档站（最常见）

```bash
# 默认 auto，无 Chrome 也能工作
web2doc https://docs.rs/tokio/latest/tokio/ -o ./tokio-docs
```

### 场景 2：SPA 文档站

```bash
web2doc https://spa.example.com/docs --mode dynamic
```

### 场景 3：只抓特定前缀

```bash
# 仅抓 /api/ 下的页面
web2doc https://docs.example.com --prefix /api/ --max-pages 100
```

### 场景 4：启用 LLM + Bundle（投喂 AI 最优方案）

```bash
export LLM_API_KEY=sk-...
web2doc https://docs.example.com --bundle --max-pages 200 -o ./ai-context

# 产物：./ai-context/_bundle.md（全文单文件）
```

### 场景 5：API 文档 → HTML 保真

```bash
# HTML 格式保留完整 tab 面板 / 表格结构 / 隐藏内容
web2doc https://api-docs.deepseek.com/zh-cn/ --format html -o ./api-docs
```

### 场景 6：断续重跑

```bash
web2doc https://docs.example.com --max-pages 100   # 首次（或中断后）
web2doc https://docs.example.com --max-pages 100   # 第二次自动续传（跳过已完成页）
web2doc https://docs.example.com --fresh            # 强制从头重抓
```

### 场景 7：Windows 下使用动态引擎

```powershell
# Windows 自动检测 Chrome（C:\Program Files\Google\Chrome\...）
web2doc https://spa.example.com --mode dynamic

# 或手动指定
web2doc https://spa.example.com --mode dynamic --chrome-path "C:\Program Files\Chromium\Application\chrome.exe"
```

---

## 输出格式

### `--format md`（默认）

适合大多数文档站。执行完整的 readability → rewrite → htmd 转换链，产出标准 GFM Markdown：
- 代码块带语言标注（` ```python `）
- 表格自动修复（缺 `<th>` → GFM 表头）
- 内链相对化、图片本地化

### `--format html`

跳过 Markdown 转换，保留 readability 提取后的原始 HTML 结构。适合：
- **API 参考文档**（请求/响应示例、多语言 tab、嵌套表格）
- 任何 Markdown 无法完整表达的结构

**注意**：HTML 格式仅保留内容结构（`<table>`、`<pre><code>`、`<h1>`-`<h6>` 等），不保留原始页面的 CSS 样式。

---

## 产物结构

```
out/
├── index.md                     # 总索引（按导航顺序排列）
├── manifest.json                # 抓取进度（断点续传依赖）
├── assets/                      # 本地化图片（sha256 命名，跨页去重）
├── _bundle.md                   # 合并产物（仅 --bundle）
└── <镜像源站路径>/*.{md,html}    # 各页面文件
```

---

## LLM 规则分析

启用后，工具在抓取首页时调用 LLM **仅一次**（站点级，不随页数线性增长），分析页面结构并返回 CSS 选择器：

- `content_selector`：正文容器
- `exclude_selectors`：需移除的噪声元素
- `nav_link_selector`：导航链接模式
- `looks_like_spa`：是否疑似 SPA

返回的规则会比内置默认规则更精准（例：对 Docusaurus 返回 `.theme-doc-markdown`）。

**降级策略**（全部失败仍可抓取）：

| 级别 | 场景 | 行为 |
| --- | --- | --- |
| 1 | 无 `LLM_API_KEY` | 使用内置默认规则 |
| 2 | LLM 返回非 JSON | 使用内置默认规则 |
| 3 | CSS 选择器非法 | 剔除非法选择器，其余保留 |
| 4 | content_selector 首页 0 命中 | 回退到默认候选链 |

---

## 覆盖率与失败口径

每次运行输出：

```
INFO run complete baseline=50 ok=19 failed=0 excluded=1  coverage=0.38  failure_rate=0.0  partial=true
```

| 指标 | 定义 | 判定 |
| --- | --- | --- |
| `coverage` | `ok / baseline` | 非截断时 < 95% → 失败 |
| `failure_rate` | `failed / discovered` | > `--max-failure-rate` → 失败 |
| `partial` | `baseline > max-pages` | **不判失败**（用户主动设限） |

> `coverage` 与 `failure_rate` 为独立判据，任一不达标即整次失败（Partial 除外）。

---

## 真实站点验证

| 验证项 | 目标站点 | 结果 |
| --- | --- | --- |
| 静态引擎 SSR | DeepSeek API 中文文档 | baseline=50, ok=19, failed=0 |
| 图片本地化 | 同上 | 12 张图，0 死链 |
| LLM DeepSeek 实调 | 同上 | 返回 `.theme-doc-markdown`，优于 fallback |
| 动态引擎 headless Chrome | 同上 | 可重复渲染，SingletonLock 已修复 |
| 代码块换行保真 | 同上 | 多行 Python 完整缩进保留 |
| 定价表格 GFM | 同上 | `td→th` + `<b>` 剥离修复 |
| 坏链口径 | fixture | port 1 拒绝 → failed=1, failure_rate=0.33 |
| robots 合规 | fixture | 被禁页排除，正常页不受影响 |

---

## 作为 AI Coding Skill 使用

把 Web2Doc 包装成 Agent Skill，让 AI 编码工具（opencode / Claude Code 等）在"需要某个库的最新官方文档"时**自动调用** `web2doc` 抓取并读取，从而减少幻觉、对齐最新 API。可分发的 skill 位于仓库内 `skills/web2doc/SKILL.md`。

### 安装

**方式一：一键安装（推荐；可让 AI 工具自动执行）**

Linux / macOS：

```bash
curl -fsSL https://raw.githubusercontent.com/LeonRust/Web2Doc/main/scripts/install-skill.sh | bash
```

Windows（PowerShell）：

```powershell
irm https://raw.githubusercontent.com/LeonRust/Web2Doc/main/scripts/install-skill.ps1 | iex
```

默认装到 `~/.claude/skills/web2doc/`——该位置会被 **Claude Code 与 opencode 同时自动发现**。也可指定 skills 父目录（bash：`... | bash -s -- <父目录>`；PowerShell：先 `$env:WEB2DOC_SKILL_DIR="<父目录>"`）。

> **让 AI 工具自己装**：直接对你的 agent 说「按 Web2Doc 的 README 安装 web2doc skill」，它会读到并执行（按自身系统选 bash 或 PowerShell）上面的一键命令——无需你手动操作。

**方式二：已克隆仓库时用 just**

```bash
just install-skill                  # -> ~/.config/opencode/skills
just install-skill ~/.claude/skills # -> Claude Code 全局
just install-skill .opencode/skills # -> 当前项目 (opencode)
```

**方式三：手动拷贝** `skills/web2doc/` 到下列任一位置：

| 工具 / 范围 | 路径 |
| --- | --- |
| opencode（项目） | `.opencode/skills/web2doc/` |
| opencode（全局） | `~/.config/opencode/skills/web2doc/` |
| Claude Code（项目 / 全局） | `.claude/skills/web2doc/` 或 `~/.claude/skills/web2doc/` |

安装后**重启 AI 工具**生效。`web2doc` 二进制无需预装——skill 首次使用时会自动下载**预编译二进制**（无需 Rust，见[安装](#安装)）。

### 触发

命中如"抓一下 xxx 的最新文档"、"给你个文档链接，先读官方文档再写"等场景时，agent 会自动运行 `web2doc <url> --bundle` 并读取产物 `_bundle.md` 作为上下文。

---

## 开发

```bash
# 门禁
just check    # cargo fmt --check && clippy -D warnings && test

# 测试
cargo test                    # 单元 + 集成（86 单元 + 5 集成）
cargo test -- --ignored       # 含网络用例（需外网/Chrome）
```

### 发布

推送 `vX.Y.Z` tag 到 GitHub 远端即触发 `.github/workflows/release.yml`，全自动完成：

- 矩阵编译 8 个 target：Linux x86_64/ARM64-gnu、x86_64/ARM64-musl、macOS Intel + Apple Silicon、Windows x86_64 + ARM64（ARM 走 GitHub 原生 ARM runner）
- 打包 `tar.gz` / `zip` + SHA256，作为附件上传到对应 GitHub Release
- `git-cliff` 按 Conventional Commits（`feat`/`fix`/`docs`…）自动生成 release notes

```bash
# 典型流程：改 Cargo.toml 版本 → 提交 → 打 tag → 推到 github 远端
git tag -a v0.1.3 -m "v0.1.3" && git push github v0.1.3
```

> tag 触发的 workflow 使用**被打 tag 那个 commit 里**的 `release.yml`/`cliff.toml`，故二者需先随分支提交。

### 项目结构

```
src/
├── cli.rs · config.rs     # 命令行解析 / 配置归一
├── obs.rs                 # 日志与可观测性
├── error.rs               # 错误类型
├── urlx.rs                # URL 规范化 / 前缀模型 / 路径映射
├── pipeline.rs            # 主编排（discover → stage A/B → index/report）
├── fetcher/               # 抓取引擎
│   ├── mod.rs · static_.rs   # trait + 静态引擎
│   ├── dynamic.rs            # 动态引擎（Chromium）
│   └── detect.rs             # Chrome 检测（macOS/Linux/Windows）
├── discover.rs            # 链接发现（sitemap → nav → BFS）
├── extract.rs             # 正文提取（readability + fallback）
├── rules.rs               # RuleSet / 回退规则
├── llm.rs                 # LLM 客户端（OpenAI 兼容，规则分析）
├── rewrite.rs             # HTML 改写（内链/图片/代码块/表格/锚点）
├── convert.rs             # HTML → Markdown + 表格修复
├── assets.rs              # 图片下载
├── writer.rs              # 文件落盘 / manifest / index / bundle
├── report.rs              # 运行报告 / 退出码
└── robots.rs              # robots.txt 合规
```

---

## 故障排查

### 日志级别

| 参数 | 作用 |
| --- | --- |
| 无 | INFO（启动 / run complete / 警告） |
| `-v` | DEBUG（discover 数据 / 每页耗时 / 阶段进度） |
| `-vv` | TRACE（依赖库详细日志） |
| `RUST_LOG=web2doc=debug` | 仅 web2doc 模块 DEBUG |

### 常见问题

**Q: `--mode dynamic` 报错“未检测到 Chrome”**  
A: 安装 Chrome / Chromium / Edge / Brave，或用 `--chrome-path` 手动指定。

**Q: 在 Linux headless 服务器上如何运行动态引擎？**  
A: 需安装 Chromium 及依赖库，并加 `--chrome-path /usr/bin/chromium`（chromiumoxide 已配置 `--no-sandbox`）。

**Q: 日志出现 `WARN WS Invalid message` / `node with weird namespace`**  
A: 无害噪声，已在 v0.1.1 中默认过滤为 ERROR 级别。如需查看可用 `RUST_LOG=chromiumoxide=warn,scraper=warn`。

**Q: `baseline=0`，什么都没抓到？**  
A: 常见于 sitemap 不含目标前缀的页面。工具已内置回退逻辑（sitemap 无结果 → nav/BFS），若仍为空请检查 `--prefix` 是否正确。

---

## 规格文档

SDD（Spec-Driven Development）四件套位于 `docs/specs/web2doc/`：

- `constitution.md` — 项目宪法（模块边界 / 安全红线 / 测试要求）
- `spec.md` — 需求规格（WHAT / WHY / Decision Log）
- `plan.md` — 技术方案（HOW / 架构 / 降级矩阵）
- `tasks.md` — 原子任务拆解（M1–M5 里程碑）

增量 Feature 文档位于 `docs/specs/<feature-id>/`。
