# Tasks · Web2Doc

- Feature ID: `web2doc`
- 状态: **v1.3（M1–M5 全部实现并真实验证；详见 git history）**
- 上游: `spec.md` v1.2 Frozen、`plan.md` v1.0 Frozen、`constitution.md`（§2/§3 已回写纳入新模块，R-1）
- 说明: 把 plan 拆为**可独立验证的原子任务**。每个任务含 输入(依赖) / 输出(交付物) / 完成标准 / 状态。

> 留痕（SDD §3）：本项目为人主导实现，以 **tasks.md + git commit** 留痕即可；`tasks.json`/`debug/` 仅在 Agent 自主调度场景引入，暂不启用（避免"工具复杂性"陷阱）。

---

## 图例与规则

- 状态：`◻ Pending` · `◐ In-Progress` · `✅ Done` · `⛔ Blocked`
- **依赖**：列出前置任务 ID；无依赖或前置已 Done 者可并行。
- **并发原则（plan §7 / SDD §6）**：组间并发（不同文件且无依赖可并行）/ 组内串行（同文件排队）/ 失败隔离（单任务失败不阻塞无依赖任务）。
- **测试网络口径（R-6，constitution §7）**：**localhost fixture server 的集成测试纳入默认 `cargo test`**；仅访问**外网/真实站点/真实浏览器**的用例标 `#[ignore]`。
- **全局 Definition of Done（每个含代码的任务都适用）**：
  1. 交付代码 + **该模块单元测试**（离线 fixture，不依赖外网）。
  2. 通过门禁 `just check`（= `cargo fmt --check && cargo clippy -- -D warnings && cargo test`）。
  3. 不违反 constitution §5 安全红线（密钥不入日志/产物；落盘路径越界校验）。
  4. 关联的 spec 验收项可被对应测试覆盖。

---

## M1 — 静态站两阶段端到端（MVP）

> 不接 LLM（用回退默认规则）、不下载图片（图片暂绝对化），打通 discover→extract→rewrite→convert→writer→report 主链 + 续传。
> 里程碑验收：A1、A3、S2、S5、S6、S9 度量。
> **注（R-5）**：M1 阶段 `baseline_total` **暂不含 robots 扣除**（robots 在 M5 接入），S1 为**临时口径**，M5 补齐至 spec §2.1 完整定义。

| ID | 任务 | 依赖 | 输出文件 | 完成标准 | 状态 |
| --- | --- | --- | --- | --- | --- |
| M1.1 | 工程骨架与门禁：`cargo init`、`Cargo.toml`(edition2021/MSRV1.75)、`.gitignore`(产物目录)、`justfile`、最简 CI | — | `Cargo.toml`/`justfile`/`.gitignore`/CI | 空骨架 `just check` 通过 | ◻ |
| M1.2 | 错误类型骨架（thiserror，§6.9） | M1.1 | `src/error.rs` | 各错误变体编译通过；应用层 anyhow 聚合 | ◻ |
| M1.3 | CLI 参数 + Config（clap §4 全集；归一/冻结；key 仅 env，禁明文） | M1.2 | `src/cli.rs`/`src/config.rs` | 解析单测；缺 URL 报错；key 不出现在日志 | ◻ |
| M1.4 | tracing 初始化与字段约定（§9 六维度；-v/-vv） | M1.1 | `src/obs.rs`(+`main.rs`) | 日志含 step/engine/error/fallback/elapsed | ◻ |
| M1.5 | URL 规范化 + 前缀模型（去 fragment/尾斜杠/解码；prefix/include-prefix 判定） | M1.2 | `src/urlx.rs` | 单测：规范化、前缀包含、去重键 | ◻ |
| M1.6 | URL→rel_path 映射算法（全量集批量；同名/query 消歧；净化；越界校验，§6.8）—— **置于共用基础模块 `urlx`，由 writer/discover 调用（C-1）** | M1.5 | `src/urlx.rs`(映射) | 专项单测：同名、query 撞名、非法字符、越界拒绝 | ◻ |
| M1.7 | `Fetcher` trait + `StaticFetcher`(reqwest) | M1.2 | `src/fetcher/mod.rs`,`static_.rs` | trait 编译；静态抓取（localhost fixture server 集成，外网用例 `#[ignore]`） | ◻ |
| M1.8 | `RuleSet` + 回退默认规则（不调 LLM；候选链首个命中） | M1.2 | `src/rules.rs` | 单测：默认 content/exclude/nav 候选链 | ◻ |
| M1.9a | discover 来源收集：sitemap/BFS/nav 三来源 + URL 规范化去重（R-4 拆分） | M1.5,M1.7,M1.8 | `src/discover.rs`(来源) | fixture 单测：三来源解析、去重 | ◻ |
| M1.9b | discover 过滤/基准/映射：前缀&URL黑名单过滤 + baseline + max-pages 截断/truncated + 调 M1.6 批量映射 + nav_order（R-4 拆分） | M1.9a,M1.6 | `src/discover.rs`(过滤/映射) | fixture 单测：baseline 计数、黑名单、截断、批量映射、nav_order | ◻ |
| M1.10 | extract：去噪(content/exclude) + dom_smoothie 兜底 + 抓取后 Excluded 二次过滤 + 收集内链/图片/视频 src + `.cache` 落盘 | M1.7,M1.8 | `src/extract.rs` | fixture 单测：§3 保留/丢弃清单、Excluded、cache 落盘 | ◻ |
| M1.11 | rewrite(M1 版)：内链相对化 + 所有资源(含图片/视频/iframe)绝对化保留 | M1.6,M1.9b,M1.10 | `src/rewrite.rs` | 单测：相对化/绝对化（唯一改写出口；用 mock 映射表独立验证） | ◻ |
| M1.12 | convert：htmd HTML→MD（保代码块/表格） | M1.10 | `src/convert.rs` | fixture 单测：代码块/表格/标题保真 | ◻ |
| M1.13 | writer 落盘：写 `.md`(镜像目录) + `index.md`(nav_order) + manifest 原子写(tmp+rename) + 状态机 | M1.6,M1.9b,M1.12 | `src/writer.rs`(落盘) | 单测：路径镜像、index 顺序、原子写、续传跳过 Written/Excluded、**续传沿用既存 rel_path（T-3）** | ◻ |
| M1.14 | pipeline 两阶段编排：A(抓取+extract+cache+Fetched)/B(rewrite+convert+write+Written) + semaphore/delay 限流 + mpsc 单写者 + 续传 + 失败隔离 | M1.7–M1.13(含 9a/9b) | `src/pipeline.rs` | 阶段切换正确；并发受限；单页失败不中断；**`--fresh` 清空 `.cache`+manifest 并全量重算映射，续传仅新页计算新映射（T-3）** | ◻ |
| M1.15 | report：RunReport 度量(`coverage=ok/baseline`,`failure_rate=failed/discovered`,partial/uncovered) + 打印 + 退出码 | M1.14 | `src/report.rs` | 单测：公式(§5.1)、退出码（非截断覆盖率<95% 或失败率>阈值→非零；截断不判失败） | ◻ |
| M1.16 | 集成测试：localhost fixture 静态站端到端(A1) + 小 max-pages 停止 + 模拟崩溃续传(A3) + 坏链失败口径(A7 部分) | M1.15 | `tests/static_e2e.rs` + `tests/fixtures/` | 默认 `cargo test` 内跑通；端到端产出 + 续传无半成品 + 度量正确 | ◻ |

---

## M2 — LLM 规则分析（站点级一次）

> 里程碑验收：S7、A5、A6。

| ID | 任务 | 依赖 | 输出文件 | 完成标准 | 状态 |
| --- | --- | --- | --- | --- | --- |
| M2.1 | LLM client：reqwest OpenAI 兼容 `POST {base_url}/chat/completions`；base_url/model/key 可配；`response_format=json_object` | M1.8 | `src/llm.rs`(client) | mock 单测（不联网）；缺 key 优雅处理 | ◻ |
| M2.2 | 规则分析：首页结构骨架提取(剥离/截断控 token) + prompt + 解析 RuleSet(serde 宽松) | M2.1 | `src/llm.rs`(analyze) | mock 响应 → 正确 RuleSet；未知字段忽略/缺失填默认 | ◻ |
| M2.3 | 降级链 4 级：调用失败/非 JSON→回退；非法 CSS 剔除；0 命中回退；静态空壳短路 | M2.2,M1.10 | `src/llm.rs`(fallback) | 分支单测全覆盖（mock） | ◻ |
| M2.4 | `looks_like_spa` 告警（static 模式 + true → 建议 --mode dynamic，计入 warnings，T-4） | M2.2 | `src/llm.rs` | 单测：触发条件 | ◻ |
| M2.5 | 接入 pipeline：首页 render→analyze(**仅 1 次**)→RuleSet 注入 discover/extract | M1.14,M2.3 | `src/pipeline.rs` | A5（调用次数与页数无关）；A6（无 key/网络→回退仍产出） | ◻ |

---

## M3 — 图片本地化与合并产物

> 里程碑验收：S3、A2、A8。

| ID | 任务 | 依赖 | 输出文件 | 完成标准 | 状态 |
| --- | --- | --- | --- | --- | --- |
| M3.1 | assets 下载：`sha256(规范化绝对 url)[..16]+ext`；Content-Type/mime_guess 兜底；`assets_seen` 规范化去重；并发信号量；产出 `src→本地相对路径`映射（**不改写 HTML**，T-5） | M1.5 | `src/assets.rs` | 单测：命名稳定、去重、扩展名兜底 | ◻ |
| M3.2 | 相对深度计算（页 rel_path 目录 → `<out>/assets/`） | M3.1,M1.6 | `src/assets.rs` | 单测：多层深度相对路径正确 | ◻ |
| M3.3 | rewrite 接入 assets：图片命中映射→本地相对；下载失败→绝对化（升级 M1.11） | M3.1,M1.11 | `src/rewrite.rs` | 单测：本地化改写 + 失败绝对化 | ◻ |
| M3.4 | bundle：`--bundle`→`<out>/_bundle.md` 按 nav_order 拼接 + 来源注释 + **图片按根层级重算**(N-7) | M1.13,M3.3 | `src/writer.rs`(bundle) | 单测(A8)：默认不生成；开启生成且图片不死链 | ◻ |

---

## M4 — 动态引擎（SPA）

> 里程碑验收：C1、S8、A4。

| ID | 任务 | 依赖 | 输出文件 | 完成标准 | 状态 |
| --- | --- | --- | --- | --- | --- |
| M4.1 | `DynamicFetcher`(chromiumoxide)：启动/复用、导航等 networkidle、取 outerHTML | M1.7 | `src/fetcher/dynamic.rs` | 真浏览器用例 `#[ignore]`；接口对齐 `Fetcher` trait | ◻ |
| M4.2 | Chrome 检测：多平台路径 + `--chrome-path` + `--mode auto/static/dynamic` 选择/降级/报错 | M4.1 | `src/fetcher/detect.rs` | 单测(mock 路径)：auto 命中/降级、dynamic 缺失报错 | ◻ |
| M4.3 | 资源模型：单页渲染超时、networkidle 阈值、tab/进程复用、SPA 路由切换（落定 §6.1 TODO） | M4.1 | `src/fetcher/dynamic.rs` | 超时不挂死；参数可配 | ◻ |
| M4.4 | 空壳告警接入(S8) + auto 降级提示 | M4.2,M1.10 | `src/pipeline.rs`/`src/extract.rs` | A4：动态站抓取；静态遇 SPA 缺失告警 | ◻ |

---

## M5 — robots、口径收尾、文档

> 里程碑验收：C9、A7、A9、A10。

| ID | 任务 | 依赖 | 输出文件 | 完成标准 | 状态 |
| --- | --- | --- | --- | --- | --- |
| M5.1 | robots：拉取/解析(texting_robots)、`is_allowed`、移出 baseline + warning、`--ignore-robots` 短路 | M1.5,M1.9b | `src/robots.rs` | 单测：Allow/Disallow、ignore 短路 | ◻ |
| M5.2 | 覆盖率/Partial/失败率口径最终收尾（baseline 含 robots 扣除补齐 R-5；RunReport 完整呈现 + 警告清单聚合） | M1.15,M5.1 | `src/report.rs` | A7/A10 度量与退出码正确；S1 达 spec §2.1 完整口径 | ◻ |
| M5.3 | 文档收尾：`README.md`(用法/示例/`LLM_API_KEY`/`--mode` 说明)、`--help` 文案 | 全部 | `README.md` | 按 README 可独立跑通 M1 场景 | ◻ |
| M5.4 | 验收集成测试：A7(失败口径)、A9(robots)、A10(Partial 截断) | M5.1,M5.2 | `tests/acceptance.rs` | 三项验收用例绿 | ◻ |

---

## 验收映射总览（tasks → spec）

| Spec 验收 | 主要任务 |
| --- | --- |
| A1 端到端+索引+覆盖率 | M1.13, M1.15, M1.16 |
| A2 正文/图片/嵌入 | M1.10, **M1.11**, M3.1–M3.3 |
| A3 续传/无半成品 | M1.13, M1.14, M1.16 |
| A4 降级/SPA 告警 | M4.1–M4.4 |
| A5 LLM 一次调用 | M2.5 |
| A6 无 key 回退 | M2.3, M2.5 |
| A7 失败口径×独立性 | M1.15, M5.2, M5.4 |
| A8 bundle | M3.4 |
| A9 robots | M5.1, M5.4 |
| A10 Partial 截断 | M1.15, M5.2, M5.4 |

> **关键路径**：M1.1→M1.2→{M1.5→M1.6, M1.7, M1.8}→M1.9a→M1.9b→M1.10→M1.11→M1.12→M1.13→M1.14→M1.15→M1.16。M2–M5 在 M1 完成后按里程碑推进；同里程碑内不同文件任务可并行。
