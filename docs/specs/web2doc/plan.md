# Plan · Web2Doc

- Feature ID: `web2doc`
- 状态: **v1.0 Frozen（实现冻结：三轮审查通过，T-1~T-5 已吸收；后续变更走 constitution §8 / Implement↔Validate 循环）**
- 上游: `spec.md` v1.2 Frozen、`constitution.md`
- 范围: 本文件描述 **HOW**。实现期若与依赖现实冲突，以 `cargo` 实际验证为准并回写本文件（constitution §1、§8）。

> 依赖均标注 *拟选型*，第一次 `cargo add` 时核实可用版本；不可用则按"备选"替换并在此记录。MSRV **1.85+**（实现期依 clap 4.6 等依赖现实确定，回写自原"暂定 1.75"）。

---

## 1. 目标映射（plan → spec）

| Spec 条目 | 由哪些模块满足 |
| --- | --- |
| S1 覆盖率 / §2.1 分母 / C3 前缀 | `discover`(文档页判定/基准全集) + `report`(覆盖率/Partial) |
| S2 去噪 / §3 边界 | `extract` + `llm`(规则) |
| S3 图片本地化 | `assets`（相对深度计算） |
| S4 视频/嵌入保真 | `rewrite` + `convert` |
| S5 结构镜像/索引 | `writer`（URL→路径映射） |
| S6 续传/无半成品 | `writer`(原子写+状态机) + `pipeline` |
| S7 LLM 站点级 | `llm`(一次规则分析) |
| S8/C1/C5 降级 | `fetcher`(双引擎) + `extract`(空壳检测) |
| S9 失败口径 | `pipeline` + `report` |
| C8 合并产物 | `writer`(可选 bundle) |
| C9 robots | `robots`(默认尊重) |

---

## 2. 架构总览（两阶段 + 状态机）

链接相对化依赖"全量 `url→rel_path` 映射"，且同名消歧需全量 URL 集，故：**discover 完成后批量计算映射**，再进入两阶段，中间态落盘以保证崩溃续传。

```
main → cli → config
                 │
                 ▼
            pipeline (编排 + 并发 + 汇总报告)
 ┌────────┬────────┬────────┬──────┬───────┬───────┬───────┬───────┬──────┐
 ▼        ▼        ▼        ▼     ▼       ▼       ▼       ▼       ▼      ▼
discover robots  fetcher   llm  extract rewrite assets convert writer report
(基准全集+        (trait) (规则一次)(去噪) (改写)  (图片)  (→MD)  (落盘+  (覆盖率
 URL黑名单+                                                      状态机) /Partial)
 批量映射)
```

数据流（一次运行）；页面状态机 `Pending → Fetched → Written`（失败转 `Failed`，抓取后判非正文页转 `Excluded`）：

```
URL ─▶ robots.load ─▶ fetcher.render(home) ─▶ llm.analyze → RuleSet(经校验, 含回退)
     ─▶ discover(sitemap|nav|bfs, 前缀+robots 过滤, URL黑名单过滤)   ← 仅 URL 级判定(T-1)
            → 初始基准全集 baseline_total
            → 批量 url→rel_path 映射(同名消歧需全量, §6.8)
            → 任务队列(max-pages 截断; 超限则标 Partial)
     ─[阶段A 抓取, 并发+限流]▶ for each task:
            fetcher.render → extract(RuleSet)
            ─ 抓取后二次文档页过滤(T-1): 无正文容器/正文过短 → 标 Excluded, 回调 baseline_total
            → 中间态落盘 <out>/.cache/<hash>.json{清洗后HTML, 内链, 资源URL}
            → manifest 标 Fetched(经单写者)
     ─[阶段B 收尾, 已知全量映射]▶ for each Fetched page:
            assets(下载图片→产出本地路径映射) → rewrite(唯一改写出口: 内链相对化/资源改写/绝对化)
            → convert(HTML→MD) → writer.write(.md) → manifest 标 Written
     ─▶ writer(index.md 按 nav_order, 可选 bundle) ─▶ report(覆盖率/Partial/成败/告警/成本)
```

> 续传按 **Written** 判定：未 Written 但有 `.cache` 的页重做阶段 B；`.cache` 缺失的页重做阶段 A；`Excluded` 页不再处理。已 Written 的页 `.md` 必存在（杜绝半成品，S6）。

---

## 3. 技术栈（拟选型）

| 用途 | crate | 理由 | 备选 |
| --- | --- | --- | --- |
| CLI | `clap`(derive) | 事实标准 | — |
| 异步 | `tokio` | 全栈异步 | — |
| HTTP | `reqwest`(rustls) | 静态抓取/下载/LLM/sitemap | — |
| 动态渲染 | `chromiumoxide` | 驱动本机 Chrome 处理 SPA | `headless_chrome` |
| async trait | 原生 async fn in trait | `Fetcher`(MSRV 1.75+) | `async-trait` |
| HTML 解析 | `scraper` | 选择器提取/链接收集/CSS 校验 | `lol_html` |
| 正文兜底 | `dom_smoothie` | readability 算法兜底 | `readable-readability` |
| HTML→MD | `htmd` | 保表格/代码块 | 自研规则 |
| robots | `texting_robots` | 解析 robots.txt | 自实现最简匹配 |
| URL | `url` | 规范化/前缀/相对化 | — |
| 序列化 | `serde`+`serde_json` | manifest/cache/LLM IO | — |
| 哈希命名 | `sha2` | 资源/缓存稳定文件名 | `blake3` |
| 并发工具 | `futures` + `tokio::sync` | 流式并发 + channel/semaphore | — |
| 错误 | `anyhow`+`thiserror` | 见 constitution §6 | — |
| 日志 | `tracing`+`tracing-subscriber` | 6 维度可观测 | — |
| MIME | `mime_guess` | URL 无扩展名时兜底扩展名 | — |

> LLM 不引专用 SDK，直接用 `reqwest` 调 `POST {base_url}/chat/completions`，保证 OpenAI 兼容与端点可换（C2）。

---

## 4. CLI 与配置

```
web2doc <URL>
  --out <DIR>              产物目录            默认 ./web2doc-out
  --prefix <PATH>          覆盖抓取前缀         默认 = URL 路径目录
  --include-prefix <PATH>  追加允许前缀(可多次)  默认 无
  --max-pages <N>          最大页数            默认 500
  --concurrency <N>        并发                默认 4
  --delay-ms <MS>          请求间隔            默认 500
  --mode <auto|static|dynamic>  抓取引擎       默认 auto
  --chrome-path <PATH>     指定 Chrome         默认 自动检测
  --base-url <URL>         LLM 端点            默认 https://api.deepseek.com
  --model <NAME>           LLM 模型            默认 deepseek-v4-flash
  --max-failure-rate <F>   失败率阈值          默认 0.20  (S9)
  --bundle                 额外输出合并文件      默认 关闭  (C8)
  --ignore-robots          忽略 robots.txt     默认 关闭(尊重) (C9)
  --fresh                  忽略既有 manifest 重抓 默认 关闭(自动续传) (S6)
  -v/-vv                   日志级别
```

- **LLM 三项配置（端点 / 模型 / 密钥）来源优先级**：CLI > 环境变量（含 `.env`）> 配置文件 > 默认。
  - 端点 / 模型：`--base-url` `--model` → `LLM_BASE_URL` `LLM_MODEL` → 配置文件 `[llm]` → 默认（`https://api.deepseek.com` / `deepseek-v4-flash`）。
  - 密钥：`LLM_API_KEY` → 配置文件 `[llm].api_key`；**绝不**接受命令行明文 key（constitution §5）。
- `.env`：启动时由 `dotenvy` 加载 CWD 下 `.env`（若存在），**不覆盖**已存在的环境变量；随后按上述优先级解析。
- 配置文件：`<config_dir>/web2doc/config.toml`（Windows = `%APPDATA%`；macOS / Linux 统一遵循 XDG = `$XDG_CONFIG_HOME` 否则 `~/.config`），仅含 `[llm]` 段，全字段可选；不存在 / 解析失败均不影响抓取（LLM 降级回退规则）。
- `config::Config` 由 CLI + 环境变量（含 `.env`）+ 配置文件归一后冻结，向下注入；业务模块不再读 env / 文件。
- 续传为默认行为（检测 `<out>/manifest.json`）；`--fresh` 显式忽略并重抓（清空 `.cache` 与 manifest，并全量重算映射，T-3）。

---

## 5. 核心数据结构

```rust
struct RuleSet {            // 来自 LLM(经校验) 或回退
    content_selector: String,
    exclude_selectors: Vec<String>,
    nav_link_selector: String,
    looks_like_spa: bool,   // 用途见 §6.4(T-4): static 模式下为 true → 建议 --mode dynamic 告警
}

struct PageTask { url: Url, rel_path: String, depth: u32 } // rel_path 由 discover 后批量映射填充(§6.8)

enum PageStatus { Pending, Fetched, Written, Failed, Excluded } // 状态机(N-1) + 抓取后判非正文(T-1)

struct PageRecord {         // 落入 manifest
    url: Url,
    rel_path: String,       // 确定性映射, §6.8（续传以既存值为准, T-3）
    status: PageStatus,     // 续传按 Written 判定
    cache: Option<String>,  // 阶段A 中间态文件 <out>/.cache/<hash>.json
    assets: Vec<String>,    // 已下载资源相对路径
    error: Option<String>,
}

struct Manifest {           // <out>/manifest.json, 原子写, 支持续传
    root_url: Url,
    prefix: String,
    rules: RuleSet,
    baseline_total: usize,  // 基准全集大小(S1 分母, 抓取后剔除 Excluded 的最终值, §2.1)
    truncated: bool,        // 初始 baseline > max-pages → Partial
    nav_order: Vec<String>, // 导航顺序(index/bundle 排序)
    pages: BTreeMap<String, PageRecord>,   // key = url
    assets_seen: BTreeMap<String, String>, // 规范化绝对 src_url → local_path (去重, N-6)
}

struct RunReport { baseline_total, discovered, ok, failed, excluded, coverage, partial, uncovered, failure_rate, llm_calls, elapsed, warnings: Vec<String> }
```

### 5.1 度量量定义（T-2，消除公式歧义）

| 量 | 定义 |
| --- | --- |
| `baseline_total` | 基准全集大小（前缀 ∩ robots 允许 ∩ 文档页）；**抓取后剔除 `Excluded` 的最终值**（S1 分母） |
| `discovered` | 实际入队页数 = `min(初始 baseline, max-pages)` |
| `ok` | 成功写出（`Written`）的页数 |
| `failed` | 抓取/处理失败（`Failed`）的页数 |
| `excluded` | 抓取后判为非正文、已从分母剔除（`Excluded`）的页数（不计入 ok/failed） |
| `coverage` | **`ok / baseline_total`**（S1，透明展示） |
| `failure_rate` | `failed / discovered`（S9） |
| `partial` / `uncovered` | `truncated` 时为真；`uncovered = baseline_total - ok`（提示提高上限用） |

---

## 6. 模块设计

### 6.1 `fetcher`（trait + 双引擎）
```rust
trait Fetcher {
    async fn render(&self, url: &Url) -> Result<RenderedPage>; // {final_url, html, status}
    fn engine(&self) -> Engine; // Static | Dynamic
}
```
- `StaticFetcher`：`reqwest` GET → 原始 HTML。
- `DynamicFetcher`：`chromiumoxide` 启动/复用浏览器，导航后等待网络空闲取 `document.outerHTML`。
- **Chrome 检测**（`mode=auto`）：`--chrome-path` 优先 → 按序探测命中即 Dynamic，否则提示并降级 Static。
  - macOS：`Google Chrome` → `Chromium` → `Edge` → `Brave` 对应 app 路径；Linux：`which google-chrome / chromium`。
  - `mode=dynamic` 且未找到 → 报错退出；`mode=static` → 不启动浏览器。
- **资源模型 TODO（M4 定）**：单浏览器多 tab vs 进程池、单页渲染超时、networkidle 阈值、SPA 路由切换检测——M4 实现前在此补定，避免返工。

### 6.2 `robots`（默认尊重 — C9）
- 启动拉取 `<origin>/robots.txt`，按 UA 解析 Disallow/Allow。
- discover 与 fetch 前对每个 URL `is_allowed` 过滤；被禁 URL 不入基准全集、不入队，计入 `RunReport.warnings`。
- `--ignore-robots` 时跳过该过滤（仍受前缀 + max-pages 约束）。

### 6.3 `discover`（基准全集 + 两级文档页判定 + 批量映射）
- **基准全集来源**（§2.1 降级链）：① `sitemap.xml`(含 index) 过滤前缀；② 无 sitemap 取首页渲染 DOM `nav_link_selector` 链接；③ 皆无取前缀内 BFS 可达集。
- **两级文档页判定（T-1，关键时序修正）**：
  - **抓取前（本模块，仅 URL 级，无需内容）**：前缀内 ∩ robots 允许 ∩ **URL 模式黑名单过滤**（排除 `/tags/`、`/categories/`、`/authors/`、`/search`、`/changelog` 等明显非正文路径）→ 定**初始** `baseline_total` 与队列。
  - **抓取后（阶段 A，§6.5）**：对已抓页用"无正文容器/正文过短"做**二次过滤**，命中者标 `Excluded`，并**从 `baseline_total` 与 coverage 分母回调剔除**。
  - 判定可被 `--include-prefix` 等显式配置覆盖。
- **批量 `url→rel_path` 映射（N-2）**：在初始基准全集**完整确定后**一次性计算（同名消歧依赖全量集，§6.8），写入各 `PageTask.rel_path` 与 `Manifest.nav_order`。**续传时以 manifest 既存 rel_path 为准，仅新页计算新映射（T-3）**。
- 队列：按 `--max-pages` 截断；若初始 `baseline_total > max-pages` → `Manifest.truncated=true`（Partial）。
- URL 规范化：去 fragment、统一尾斜杠、解码、按 §6.8 处理 query。

### 6.4 `llm`（仅规则分析，站点级一次 — S7）
- 输入：首页渲染 HTML 的**结构骨架**（剥离长文本/截断，控 token）。
- **静态空壳短路（N-5）**：若 `mode=static` 且首页疑似 SPA 空壳（正文骨架为空），**跳过 LLM 调用**直接用回退规则。
- 输出强约束 JSON（`response_format=json_object`），**严格匹配 `RuleSet` 四字段**：
  ```json
  {
    "content_selector": "main article",
    "exclude_selectors": ["nav", ".sidebar", "footer", ".breadcrumb", ".pagination", ".edit-this-page"],
    "nav_link_selector": "nav a, aside a, .sidebar a",
    "looks_like_spa": true
  }
  ```
- **`looks_like_spa` 的用途（T-4）**：当 `mode=static` 且该值为 `true` 时，输出告警"该站疑似 SPA，建议改用 `--mode dynamic` 获取完整内容"（计入 `RunReport.warnings`）；其它情况仅供参考。
- **完整降级链（覆盖 A6）**：
  1. 调用失败/被短路（无 key/网络/非 JSON/空壳）→ 整体用回退默认 `RuleSet`。
  2. JSON 合法 → serde 反序列化（**未知字段忽略、缺失字段填回退默认**）。
  3. `scraper` 校验每个选择器是否合法 CSS → **剔除非法选择器**。
  4. 首页验证 `content_selector` 命中非空正文；0 命中/正文过短 → 该项**回退候选链**。
  - 全程仍只 1 次（或 0 次）LLM 调用，不破坏 S7。
- **回退默认**：content `["main","article","[role=main]",".markdown-body",".content","#content"]` 首个命中；exclude 噪声选择器集；nav `"nav a, aside a, .sidebar a, .toc a, .menu a"`。

### 6.5 `extract`（去噪 + 抓取后过滤 → §3 边界，阶段 A）
- 用 `content_selector` 取正文容器；移除 `exclude_selectors` 子树；命中为空或正文过短 → `dom_smoothie` 兜底。
- **抓取后文档页二次过滤（T-1）**：兜底后仍无正文容器/正文过短 → 标 `Excluded`（回调分母，§6.3），不产出 `.md`。
- **SPA 空壳检测**（S8）：静态引擎下正文极短且存在 `#root/#app` 空容器 → 记 `RunReport.warnings`。
- 产出**清洗后 HTML 片段**并收集内链 `<a href>`、图片 `<img src>`、`<video>/<iframe> src`；落盘 `.cache`，**此阶段不改写**。

### 6.6 `assets`（仅图片 — N3，阶段 B；只产映射不改写 — T-5）
- 下载正文图片到 `<out>/assets/`；文件名 = `sha256(规范化绝对 src_url)[..16] + ext`。
- **去重 key（N-6）**：以**规范化后的绝对 URL** 为 `assets_seen` 键，统一 `//cdn/x`、`/x`、`https://cdn/x`。
- **ext 来源**：优先 URL 扩展名；无则用响应 `Content-Type` 经 `mime_guess` 兜底。
- **产出**：`src_url → 本地相对路径` 映射（相对深度 = 从该页 `rel_path` 目录到 `<out>/assets/`，如 `../../assets/x.png`）。**本模块不改写 HTML**；改写统一由 `rewrite` 执行（T-5）。下载失败的图片不入映射，由 rewrite 绝对化保留原 URL。

### 6.7 `rewrite`（唯一链接/资源改写出口，阶段 B — T-5）
- 输入：清洗后 HTML + 全量 `url→rel_path` 映射 + `assets` 的 `src_url→本地路径` 映射。
- **所有 `<a href> / <img src> / <video> / <iframe>` 的改写都在此完成**：
  - 内链指向已抓本地页 → 改为相对 `rel_path`；图片命中 assets 映射 → 改为本地相对路径；
  - 外链 / 未抓页 / 未本地化资源（下载失败图片、视频、iframe）→ **绝对化保留**。

### 6.8 `writer`（落盘 — S5/S6）
- **URL→文件路径映射（确定性，基于完整 URL 集批量计算；算法实现于共用基础模块 `urlx`，由 writer 与 discover 调用 — C-1）**：
  1. 取 path（去 query/fragment），按段净化非法/保留字符。
  2. 末尾 `/` 或目录页 → `…/index.md`。
  3. 含扩展名（`.html` 等）→ 改 `.md`；无扩展名末段 → `<seg>.md`。
  4. **同名消歧**：某路径既是页面又是其它页面父目录（`/guide` 与 `/guide/x` 并存）→ 统一 `/guide/index.md`（**需全量集判定**）。
  5. **query 消歧**：默认忽略；仅 query 不同会撞同一文件 → 追加 `-<query_hash>` 后缀。
  6. **越界校验**：规范化后路径必须仍在 `<out>` 内（constitution §5）。
  - **续传一致性（T-3）**：续传时既有页沿用 manifest 的 `rel_path`，仅对新出现页计算映射；`--fresh` 才全量重算。
- 总索引 `<out>/index.md`：按 `nav_order` 排序，回退路径字典序。
- `--bundle` → `<out>/_bundle.md`：按同一顺序拼接，每段附来源 URL 注释；**bundle 内图片路径按根层级重算为 `assets/…`**（N-7）。
- **manifest 原子写**：写 `<out>/manifest.json.tmp` 后 `rename` 覆盖。
- **目录**：中间态在 `<out>/.cache/`（结束可清理）；`assets/`、`index.md`、`manifest.json` 为固定名（constitution §4）。

### 6.9 `report` / `error`
- `report`：汇总 `RunReport`，**覆盖率(`ok/baseline_total`)/Partial 与失败率独立呈现**；非截断的覆盖率<95% 或失败率>阈值 → 非零退出；纯 Partial（截断）不视为失败但显式提示。
- `error`：`thiserror` 定义 `FetchError/ExtractError/LlmError/RobotsError/IoError…`；应用层 `anyhow` 聚合。

---

## 7. 并发与限流

- 阶段 A/B 各用 `futures::stream::buffer_unordered(concurrency)`；图片下载并发受信号量约束。
- 全局 `Semaphore` 限并发；每网络请求前 `sleep(delay_ms)`（C4）。
- **manifest 单写者**：页面状态变更经 `tokio::mpsc` 汇聚到**唯一 writer task** 串行落盘 + 原子写。
- 续传：启动读 manifest，跳过 `Written` 与 `Excluded`；`Fetched` 未 `Written` 的从 `.cache` 续做阶段 B；缺 `.cache` 的重做阶段 A；`rel_path` 沿用既存值（T-3）。
- SDD 并发映射：页面任务无文件冲突 → 组间并发；manifest 串行单写 → 组内串行；单页失败仅记录 → 失败隔离。

---

## 8. 降级与容错矩阵

| 情形 | 行为 |
| --- | --- |
| 无 Chrome（auto） | 告警 + 降级 Static（S8） |
| mode=dynamic 无 Chrome | 报错退出 |
| 静态空壳/无 LLM key/网络 | 短路或回退默认规则继续（A6、N-5） |
| static 模式 + LLM 判 SPA | 告警"建议 --mode dynamic"（T-4） |
| LLM 合法 JSON 但选择器非法/0 命中 | 剔除 → 回退候选链 |
| robots 禁止某 URL | 移出基准全集 + 跳过 + warning（除非 --ignore-robots） |
| 抓取后判为非正文页 | 标 `Excluded`，从分母剔除（T-1），不产出 .md |
| 单页抓取/解析失败 | 记 `Failed`，继续（失败隔离） |
| 图片下载失败 | rewrite 绝对化保留原 URL（§3，S3 容忍 ≤5%） |
| **baseline ≤ max-pages 且覆盖率<95%** | 非零退出 + 报告（S1 失败） |
| **初始 baseline > max-pages（截断）** | 标 Partial，**不判失败**，透明提示未覆盖数（§2.1、A10） |
| 失败率 > 阈值 | 仍写出已成功页，非零退出 + 失败清单（S9） |
| 进程中途崩溃 | 重跑按 Written 续传，无半成品（S6、N-1） |

---

## 9. 可观测性（对齐 constitution §6 / SDD 六维度）

`tracing` 字段：`target_url`、`step`(robots/discover/render/extract/assets/rewrite/convert/write)、`engine`、`error`、`fallback`、`elapsed_ms`+`llm_calls`。结束打印 `RunReport`（覆盖率/Partial 与失败率分列）。

> 治理范围说明：本工具为单次运行 CLI，SDD §7 的 Eval/Goal-Driven 暂不引入（避免"工具复杂性"陷阱）；observability + manifest 留痕已满足"可复盘/可重复"。

---

## 10. 测试策略（constitution §7）

- 单元（离线 fixtures，`tests/fixtures/*.html` + `*.xml`）：
  - `discover`：sitemap 解析、**URL 黑名单过滤**、前缀/robots 过滤、去重、初始 baseline 计数、max-pages 截断与 truncated、批量映射、nav_order、续传 rel_path 沿用。
  - `robots`：Allow/Disallow 匹配、--ignore-robots 短路。
  - `extract`：选择器去噪、readability 兜底、**抓取后 Excluded 二次过滤**、空壳检测、§3 保留/丢弃清单、`.cache` 落盘。
  - `llm`：JSON 解析 + 降级链（静态空壳短路、非法选择器/0 命中、looks_like_spa 告警），mock 不联网。
  - `assets`：哈希命名稳定、规范化去重、扩展名兜底、相对深度、**仅产映射不改写**。
  - `rewrite`：内链相对化、图片本地化改写、外链/未本地化资源绝对化（唯一改写出口）。
  - `writer`：URL→路径映射（同名/query 消歧/越界）、index 按 nav_order、bundle 图片根层级重算、manifest 原子写、续传按 Written/Excluded 跳过。
  - `report`：`coverage=ok/baseline_total`、Partial、failure_rate、退出码（截断不判失败、Excluded 不计 fail）。
- 集成：本地静态 fixture 站点跑通两阶段 pipeline + 模拟崩溃续传（A1/A3/A7/A10）。
- 网络/浏览器用例标 `#[ignore]`，不入默认 `cargo test`。
- 交付门禁：`cargo fmt --check && cargo clippy -- -D warnings && cargo test`（clippy 仅本地包，N-8）；M1 即落 `justfile`/最简 CI。

---

## 11. 里程碑（先难后易：先打通主链，再补引擎）

| M | 目标 | 验收 |
| - | --- | --- |
| **M1** | 骨架+静态引擎+回退规则+discover(基准全集/URL黑名单/批量映射)+extract(.cache+Excluded)+rewrite+convert+writer(映射/原子写/状态机)+report：静态站两阶段端到端 + 续传 | A1、A3、S2、S5、S6、S9 度量 |
| **M2** | LLM 规则分析（站点级一次）+ 降级链 + 空壳短路 + looks_like_spa 告警 | S7、A5、A6 |
| **M3** | assets 图片本地化（去重/相对深度/扩展名）+ rewrite 绝对化 + bundle | S3、A2、A8 |
| **M4** | 动态引擎（chromiumoxide）+ Chrome 检测 + 资源模型 + 空壳告警 | C1、S8、A4 |
| **M5** | robots + 覆盖率/Partial/失败率口径收尾 + RunReport + 文档收尾 | C9、A7、A9、A10 |

> M1 即可用 MVP，且已含确定性映射、原子续传、状态机、两级文档页判定四个易翻车点。

---

## 12. 风险与缓解

| 风险 | 缓解 |
| --- | --- |
| `htmd`/`dom_smoothie` 能力或 API 不及预期 | M1 留备选（自研规则/`readable-readability`），实现期 cargo 验证 |
| 单套选择器对异构站点泛化差 → 威胁 S2 | readability 兜底 + S2 抽检 ≥10 页；必要时按区域缓存多套规则（M2+ 评估） |
| 文档页判定误杀/漏判 | 抓取前 URL 黑名单 + 抓取后正文容器二次过滤 + `--include-prefix` 覆盖；专项单测 |
| chromiumoxide 跨环境不稳 | 与静态引擎解耦，失败可降级；M4 才引入 |
| URL→路径映射边界（同名/query/非法字符）| §6.8 确定性算法（全量集）+ 专项单测 |
| 中间态/manifest 损坏或半成品 | `.cache` 落盘 + 状态机 + 单写者 + 原子写（§7、§6.8） |
| 续传时站点变化致 rel_path 漂移 | 既存页沿用 manifest rel_path，仅新页新映射（T-3） |
| 大站耗时/被限速/超 max-pages | 并发+间隔可配，Partial 透明提示，续传 |
| 路径穿越 | writer 越界校验（constitution §5） |

---

## 13. 决策点（已经人确认）

1. **MVP 范围**：✅ M1（静态端到端）为第一个可交付里程碑。
2. **依赖取舍**：✅ `htmd` + `dom_smoothie` 主选，实现期验证，不行切备选。
3. **默认 LLM 端点**：✅ `https://api.deepseek.com` / `deepseek-v4-flash`（端点 / 模型 / 密钥支持 CLI > env > 配置文件 > 默认覆盖，密钥仅 env / 文件）。
4. **资源命名**：✅ `sha256(规范化绝对 url)[..16]+ext`。
5. **robots**：✅ 默认尊重 `robots.txt` + `--ignore-robots` 逃生口。
6. **max-pages 截断口径**：✅ baseline > max-pages 标 Partial、不判失败、透明提示（§2.1）。

> **v1.0 Frozen**：规格审查结束（三轮），进入 Implement↔Validate；后续仅微小变更或实现暴露的真实问题才回写本文件（constitution §8）。
