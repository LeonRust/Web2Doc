# Web2Doc

把在线文档站抓取为本地结构化 **Markdown**，用作 AI Coding 工具的文档上下文约束 —— 避免模型依赖过时训练数据写出错误代码。

## 特性

- **双引擎抓取**：静态（reqwest）覆盖 SSR / 静态站；`--mode dynamic` 用 headless Chrome（chromiumoxide）渲染 SPA；`auto` 自动检测 Chrome 并在缺失时降级为静态 + 告警。
- **端到端抓取**：给定文档站首页 URL，自动跨页发现并抓取整站文档（`sitemap` → 导航 → 前缀 BFS 降级链）。
- **正文去噪**：readability 算法提取正文，剔除导航 / 侧栏 / 页脚 / 广告等噪声；非正文页 / 空壳自动排除。
- **LLM 规则分析（可选）**：配置 `OPENAI_API_KEY` 时由 LLM 站点级**一次**分析正文/导航选择器（4 级降级链；无 key 自动回退默认规则）。
- **图片本地化**：正文图片下载到 `assets/`，引用改写为相对路径（无死链）。
- **内链相对化**：站内链接改写为本地相对路径，离线可跳转。
- **代码块保真**：保留代码换行 / 缩进、表格、标题（对喂 AI 的代码示例至关重要）。
- **断点续传**：中断后重跑跳过已完成页，不产生半成品。
- **robots 合规**：默认尊重 `robots.txt`（可 `--ignore-robots`）。
- **可信口径**：覆盖率 / 失败率独立度量；`max-pages` 截断标记 Partial 且不判失败；警告聚合。
- **合并产物**：`--bundle` 生成全文单文件，便于整体投喂 AI。

## 安装

```bash
cargo build --release   # 二进制：target/release/web2doc
```

## 用法

```bash
web2doc <URL> [选项]

# 示例：抓取 DeepSeek API 中文文档（前 50 页，输出到 ./deepseek-docs）
web2doc https://api-docs.deepseek.com/zh-cn/ --max-pages 50 --out ./deepseek-docs

# SPA 站点：用动态引擎（headless Chrome 渲染）
web2doc https://some-spa-docs.example.com/ --mode dynamic

# 启用 LLM 规则分析（OpenAI 兼容）
export OPENAI_API_KEY=sk-...   # 兼容 DEEPSEEK_API_KEY
web2doc https://docs.example.com/
```

### 常用选项

| 选项 | 说明 | 默认 |
| --- | --- | --- |
| `--out <DIR>` | 产物输出目录 | `./web2doc-out` |
| `--prefix <PATH>` | 覆盖抓取前缀 | URL 路径目录 |
| `--include-prefix <PATH>` | 追加允许前缀（可多次） | 无 |
| `--max-pages <N>` | 最大页数上限 | 500 |
| `--concurrency <N>` | 并发数 | 4 |
| `--delay-ms <MS>` | 请求间隔（礼貌） | 500 |
| `--mode <auto\|static\|dynamic>` | 抓取引擎（auto 自动检测 Chrome） | auto |
| `--chrome-path <PATH>` | 指定 Chrome 可执行文件 | 自动检测 |
| `--base-url <URL>` | LLM 端点（OpenAI 兼容） | `https://api.deepseek.com` |
| `--model <NAME>` | LLM 模型 | `deepseek-chat` |
| `--bundle` | 额外输出合并文件 `_bundle.md` | 关闭 |
| `--ignore-robots` | 忽略 robots.txt | 关闭（尊重） |
| `--fresh` | 忽略既有进度重新抓取 | 关闭（自动续传） |
| `-v` / `-vv` | 日志详细度 | INFO |

完整选项见 `web2doc --help`。

> **环境变量**：`OPENAI_API_KEY`（或 `DEEPSEEK_API_KEY`）启用 LLM 规则分析；动态引擎需本机已装 Chrome / Chromium / Edge / Brave。

## 产物结构

```
out/
├── index.md          # 总索引（按导航顺序）
├── manifest.json     # 抓取进度（断点续传）
├── assets/           # 本地化图片（sha 命名，去重）
├── _bundle.md        # 合并产物（仅 --bundle）
└── <镜像源站路径>/*.md
```

## 里程碑（v0.1.0，全部完成）

- ✅ **M1** 静态 / SSR 端到端
- ✅ **M2** LLM 规则分析 + 4 级降级链
- ✅ **M3** 图片本地化 + bundle
- ✅ **M4** 动态引擎（SPA / headless Chrome）
- ✅ **M5** robots 合规 + 口径收尾 + 文档

真实站点验证：DeepSeek API 中文文档端到端通过（静态 50 页 / 0 失败 / 图片本地化 0 死链；动态引擎 headless Chrome 渲染成功）。

## 开发

```bash
just check   # cargo fmt --check + clippy -D warnings + test
```

规格文档（SDD）见 `constitution.md` 与 `docs/specs/web2doc/`（constitution + spec + plan + tasks）。
