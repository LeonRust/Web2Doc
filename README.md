# Web2Doc

把在线文档站抓取为本地结构化 **Markdown** 或 **HTML**，用作 AI Coding 工具的文档上下文约束 —— 避免模型依赖过时训练数据写出错误代码。

## 快速开始

```bash
# 安装（需要 Rust 1.85+）
cargo build --release   # 二进制：target/release/web2doc

# 基础用法
web2doc <URL> [选项]

# 抓取 DeepSeek API 中文文档（前 50 页，输出到 ./deepseek-docs）
web2doc https://api-docs.deepseek.com/zh-cn/ --max-pages 50 --out ./deepseek-docs
```

## 抓取引擎

Web2Doc 使用**双引擎**架构，自动适配不同类型的文档站。

### 静态引擎（默认）

基于 `reqwest` 的纯 HTTP 抓取器。适用于**服务端渲染（SSR）或静态生成（SSG）**的文档站，包括：

- **Docusaurus**（如 api-docs.deepseek.com）
- **VitePress**、**mdBook**、**Sphinx**、**GitBook**、**Docsify** 等
- 任何 HTML 内容在首屏就完整的站点

静态引擎速度快、资源消耗低，是大多数场景的最佳选择。

### 动态引擎（headless Chrome）

基于 `chromiumoxide` 驱动 **headless Chrome / Chromium** 渲染页面后抓取 DOM。

适用于**纯客户端渲染（SPA）**的文档站，这些站点首屏 HTML 是空壳，内容由 JavaScript 动态生成。动态引擎会等待页面完成导航后再取渲染后的完整 HTML（含 200ms SPA 异步渲染缓冲），有 30 秒超时保护。

**Chrome 自动检测（macOS / Linux）**

工具在 `--mode auto`（默认）下自动检测本机 Chrome，按以下优先级扫描：

| 平台 | 检测路径 |
| --- | --- |
| macOS | `/Applications/Google Chrome.app/…/Google Chrome` → Chromium → Edge → Brave |
| Linux | `/usr/bin/google-chrome` → `/usr/bin/chromium` → `/usr/bin/chromium-browser` → `/usr/bin/microsoft-edge` |

可通过 `--chrome-path` 手动指定可执行文件路径。

**模式说明**

| `--mode` | 行为 |
| --- | --- |
| `auto`（默认） | 有 Chrome → 动态引擎；无 Chrome → 静态引擎 + 告警 |
| `static` | 强制静态引擎，不启动浏览器 |
| `dynamic` | 强制动态引擎；无 Chrome 则报错退出 |

### 引擎对比

| | 静态引擎 | 动态引擎 |
| --- | --- | --- |
| 依赖 | 无（纯 Rust） | 需本机安装 Chrome / Chromium / Edge / Brave |
| 适用站点 | SSR / SSG（Docusaurus, VitePress, mdBook…） | SPA（客户端渲染） |
| 速度 | 快 | 较慢（每页启动 tab 渲染） |
| 资源 | 低 | 较高（浏览器进程） |

> **提示**：大多数主流文档站是 SSR/SSG（包括 DeepSeek API 文档），直接用默认的 `auto` 即可 —— 有 Chrome 自动启用更强的渲染能力，没有 Chrome 也能正常工作。

## LLM 规则分析（可选）

配置 `OPENAI_API_KEY` 环境变量后，工具会在抓取首页时调用 LLM **一次**（站点级），自动分析页面的正文选择器、噪声选择器和导航链接模式。LLM 返回的规则会比内置默认规则更精准（例如对 Docusaurus 站点返回 `.theme-doc-markdown`）。

支持的 LLM 端点：OpenAI 兼容接口（默认 `https://api.deepseek.com`，模型 `deepseek-chat`）。可通过 `--base-url` / `--model` 切换。

内置 **4 级降级链**：LLM 调用失败 / 返回非法 JSON / CSS 选择器无效 / 首页 0 命中 → 自动回退到默认规则，不会中断抓取。

## 常用选项

| 选项 | 说明 | 默认 |
| --- | --- | --- |
| `--out <DIR>` | 产物输出目录 | `./web2doc-out` |
| `--prefix <PATH>` | 覆盖抓取前缀 | URL 路径目录 |
| `--include-prefix <PATH>` | 追加允许前缀（可多次） | 无 |
| `--max-pages <N>` | 最大页数上限 | 500 |
| `--concurrency <N>` | 并发数 | 4 |
| `--delay-ms <MS>` | 请求间隔（礼貌） | 500 |
| `--mode <auto\|static\|dynamic>` | 抓取引擎 | auto |
| `--chrome-path <PATH>` | 指定 Chrome 可执行文件 | 自动检测 |
| `--format <md\|html>` | 输出格式（html 保留表格/隐藏 tab 完整结构） | md |
| `--base-url <URL>` | LLM 端点（OpenAI 兼容） | `https://api.deepseek.com` |
| `--model <NAME>` | LLM 模型 | `deepseek-chat` |
| `--bundle` | 额外输出合并文件 `_bundle.md` | 关闭 |
| `--ignore-robots` | 忽略 robots.txt | 关闭（尊重） |
| `--max-failure-rate <F>` | 失败率阈值（超过判整次失败） | 0.20 |
| `--fresh` | 忽略既有进度重新抓取 | 关闭（自动续传） |
| `-v` / `-vv` | 日志详细度 | INFO |

完整选项见 `web2doc --help`。

### 环境变量

| 变量 | 说明 |
| --- | --- |
| `OPENAI_API_KEY` | LLM API Key（兼容 `DEEPSEEK_API_KEY`）—— 配置后才启用 LLM 规则分析 |
| — | 动态引擎需本机已装 Chrome / Chromium / Edge / Brave |

## 用法示例

```bash
# SSR/SSG 站点（默认，最快）
web2doc https://api-docs.deepseek.com/zh-cn/ --max-pages 50

# SPA 站点（强制 Chrome 渲染）
web2doc https://some-spa.example.com/docs/ --mode dynamic

# 启用 LLM 分析 + 合并产物（整站单文件投喂 AI）
export OPENAI_API_KEY=sk-...
web2doc https://docs.example.com/ --bundle

# 复杂 API 文档 → HTML 保真（保留所有 tab / 代码块 / 表格结构）
web2doc https://api-docs.deepseek.com/zh-cn/ --format html

# 只抓取指定前缀下的页面
web2doc https://docs.example.com/ --prefix /api-reference/

# 断续重跑（跳过已完成页）
web2doc https://docs.example.com/    # 第二次自动续传
web2doc https://docs.example.com/ --fresh   # 强制重抓
```

## 产物结构

```
out/
├── index.md                     # 总索引（按导航顺序）
├── manifest.json                # 抓取进度（断点续传）
├── assets/                      # 本地化图片（sha256 命名，跨页去重）
├── _bundle.md                   # 合并产物（仅 --bundle）
└── <镜像源站路径>/*.{md,html}    # 各页面（格式由 --format 决定）
```

## 特性

- **双引擎抓取**：静态（reqwest）覆盖 SSR/SSG 站 · 动态（headless Chrome）渲染 SPA · `auto` 自动检测 Chrome 并降级
- **端到端发现**：`sitemap` → 导航 → 前缀 BFS 三级降级链，自动发现整站页面
- **内链全面相对化**：已抓取页→本地路径，未抓取的站内同前缀页→自动推算路径，外站→保留绝对
- **正文去噪**：readability 算法提取正文，剔除导航/侧栏/页脚/广告；非正文页/空壳自动排除
- **LLM 规则分析（可选）**：`OPENAI_API_KEY` 配置后站点级一次分析（4 级降级链）
- **图片本地化**：下载到 `assets/`，sha256 命名，manifest 级跨页去重缓存，引用改写为相对路径（无死链）
- **代码块保真**：拍平 Prism token 行内元素 + `<br>`→`\n`，保留多行缩进
- **表格修复**：自动将 `<td>` 首行转为 `<th>`，剥离外层 `<b>` 标签，生成标准 GFM 表格
- **断点续传**：状态机（Pending→Fetched→Written）+ 原子写 manifest，中断后重跑不产生半成品
- **robots 合规**：默认尊重 `robots.txt`（可 `--ignore-robots`）
- **可信口径**：覆盖率 / 失败率独立度量 · `max-pages` 截断标记 Partial 且不判失败 · 警告聚合 · `--max-failure-rate` 可控阈值
- **HTML 输出**：`--format html` 跳过 htmld 转换，保留复杂页面完整结构（对 API 文档/多 tab 页面推荐）
- **合并产物**：`--bundle` 生成全文单文件，图片路径根层级重算

## 真实站点验证

DeepSeek API 中文文档全功能端到端通过：

| 验证项 | 结果 |
| --- | --- |
| 静态引擎（SSR 文档站） | baseline=50, ok=19, failed=0 |
| 图片本地化 | 12 张图本地化，0 死链 |
| LLM DeepSeek API 实调 | 返回站点特定选择器 `.theme-doc-markdown`（优于 fallback） |
| 动态引擎（headless Chrome） | 可重复渲染，SingletonLock 已修复 |
| 代码块换行保真 | 多行 Python 代码保留完整缩进 |
| 定价表格 | GFM 表格正常生成（`td→th` + 剥离 `<b>` 修复） |
| 坏链失败口径 | port 1 连接拒绝 → failed=1, failure_rate=0.33, exit_code=1 |
| robots 合规 | robots.txt 加载，被禁页不抓取 |

## 开发

```bash
just check   # cargo fmt --check + clippy -D warnings + test（77 单元 + 5 集成全绿）
```

规格文档（SDD）见 `constitution.md` 与 `docs/specs/web2doc/`（constitution + spec + plan + tasks）。
