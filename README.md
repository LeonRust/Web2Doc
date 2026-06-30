# Web2Doc

将在线文档站点抓取为本地结构化 Markdown/HTML，供 AI Coding 工具作为上下文约束使用。

[![Rust](https://img.shields.io/badge/rust-1.85+-orange)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue)](./LICENSE)
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

```bash
git clone https://github.com/<user>/web2doc
cd web2doc
cargo build --release

# 二进制位于 target/release/web2doc
# 可选：安装到 $PATH
cargo install --path .
```

---

## 快速开始

```bash
# 抓取任意文档站（默认静态引擎，无需 Chrome）
web2doc https://api-docs.deepseek.com/zh-cn/ --max-pages 50 -o ./docs

# SPA 站点需要开启动态引擎
web2doc https://spa-docs.example.com --mode dynamic

# 启用 LLM 分析 + 合并产物（投喂 AI）
export OPENAI_API_KEY=sk-...
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
                   ┌───────▼────────┐
                   │   --mode ?      │
                   │ auto/static/    │
                   │ dynamic         │
                   └───┬─────┬───────┘
           ┌───────────┘     └───────────┐
           ▼                             ▼
  ┌─────────────────┐          ┌──────────────────┐
  │  Static Engine   │          │  Dynamic Engine   │
  │  (reqwest)       │          │  (chromiumoxide)  │
  │                  │          │                   │
  │  SSR / SSG 站点   │          │  SPA / CSR 站点    │
  │  · Docusaurus    │          │  · 客户端渲染       │
  │  · VitePress     │          │  · 需 Chrome       │
  │  · mdBook        │          │  · headless 模式   │
  │  · GitBook       │          │                   │
  └────────┬─────────┘          └────────┬──────────┘
           │                             │
           └───────────┬─────────────────┘
                       ▼
             ┌───────────────────┐
             │  Pipeline         │
             │  · discover      │
             │  · readability   │
             │  · rewrite       │
             │  · convert/write │
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

| 选项 | 类型 | 默认值 | 说明 |
| --- | --- | --- | --- |
| `--base-url <URL>` | string | `https://api.deepseek.com` | LLM 端点（OpenAI 兼容） |
| `--model <NAME>` | string | `deepseek-chat` | LLM 模型名 |

### 日志

| 选项 | 说明 |
| --- | --- |
| `-v` | DEBUG 级别（显示每页抓取耗时、阶段进度） |
| `-vv` | TRACE 级别（含依赖库详细日志） |

---

## 环境变量

| 变量 | 说明 |
| --- | --- |
| `OPENAI_API_KEY` | LLM API Key；设置后才启用 LLM 规则分析 |
| `DEEPSEEK_API_KEY` | 兼容别名 |
| `RUST_LOG` | 覆盖默认日志级别（例 `RUST_LOG=web2doc=debug`） |

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
export OPENAI_API_KEY=sk-...
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
| 1 | 无 `OPENAI_API_KEY` | 使用内置默认规则 |
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

## 开发

```bash
# 门禁
just check    # cargo fmt --check && clippy -D warnings && test

# 测试
cargo test                    # 单元 + 集成（80 单元 + 5 集成）
cargo test -- --ignored       # 含网络用例（需外网/Chrome）
```

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
