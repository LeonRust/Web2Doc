# Constitution · Web2Doc 项目宪法

> 本文件是 Web2Doc 不可违背的全局约束（SDD 中的"宪法"）。所有 `spec.md` / `plan.md` / 代码实现都必须遵守。与本文件冲突的实现一律视为错误。
> 信条：**Spec 是唯一真实来源，人定义 WHAT，AI 实现 HOW。脚手架 > 模型。**

适用范围：本仓库内所有代码与文档。

---

## 1. 技术基线

- 语言：Rust（edition 2021，stable 工具链）。
- 形态：单一 CLI 二进制 `web2doc`，异步运行时统一用 `tokio`。
- 平台：macOS 优先，代码须保持跨平台可编译（不写死平台路径，平台差异集中在专门模块）。
- 不重复造轮子：优先使用成熟 crate；引入新依赖需在 `plan.md` 说明理由。

---

## 2. 目录结构约定

```
web2doc/
├── Cargo.toml
├── constitution.md              # 本文件
├── docs/specs/<feature>/        # SDD 文档：spec.md / plan.md / tasks.md
├── src/
│   ├── main.rs                  # 入口：仅解析 CLI、装配、启动
│   ├── lib.rs                   # 库根：聚合 pub 模块（lib + bin 结构）
│   ├── cli.rs                   # 命令行参数定义（clap）
│   ├── config.rs                # 运行配置（合并 CLI + 环境变量）
│   ├── obs.rs                   # tracing / 可观测性初始化（共用基础）
│   ├── urlx.rs                  # URL 规范化、前缀模型、URL→rel_path 映射（共用基础）
│   ├── pipeline.rs              # 编排：发现→抓取→提取→改写→转换→写出
│   ├── robots.rs                # robots.txt 合规（默认尊重）
│   ├── discover.rs              # 链接发现 + 基准全集（sitemap / 导航 / 前缀爬取）
│   ├── fetcher/                 # 抓取引擎（trait + 静态/动态实现）
│   ├── rules.rs                 # RuleSet 与回退默认规则
│   ├── llm.rs                   # OpenAI 兼容客户端（仅规则分析）
│   ├── extract.rs               # 正文提取（选择器 + readability 兜底）
│   ├── rewrite.rs               # 链接/资源改写（唯一改写出口）
│   ├── assets.rs                # 图片下载（稳定命名 + 去重）
│   ├── convert.rs               # HTML → Markdown
│   ├── writer.rs                # 产物落盘（镜像目录 + 索引 + manifest）
│   ├── report.rs                # 运行报告与退出码
│   └── error.rs                 # 错误类型
└── tests/                       # 集成测试 + fixtures
```

- 产物默认输出到运行时指定的 `--out` 目录，**不写入仓库**（须在 `.gitignore`）。
- 抓取进度 `manifest.json` 与产物同目录，供断点续传。

---

## 3. 模块边界与依赖方向

- 依赖单向，**禁止循环依赖**。允许方向：
  ```
  main → cli/config → pipeline → {robots, discover, fetcher, llm, extract, rewrite, assets, convert, writer, report}
  ```
- `urlx`（URL 工具）、`rules`（RuleSet 类型）、`obs`（日志初始化）、`error`（错误类型）为被多模块依赖的**共用基础模块**，不得反向依赖业务模块，亦不构成环。
- `fetcher` 必须以 trait 抽象，静态引擎与动态引擎为可替换实现；上层只依赖 trait，不依赖具体引擎。
- `llm` 仅承担"规则分析"（输入页面结构，输出选择器/判定规则），**禁止**承担逐页正文提取或多步自主决策（决策层级停在 Prompt 层，不上推到 Agent 层）。
- 业务模块不直接读环境变量；所有外部输入经 `config` 归一后注入。

---

## 4. 命名规则

- 模块/文件/函数/变量：`snake_case`；类型/Trait/枚举：`UpperCamelCase`；常量：`SCREAMING_SNAKE_CASE`。
- 二进制与命令名：`web2doc`。
- 本地资源文件命名稳定可复现（基于源 URL 哈希 + 原扩展名），避免随机导致重复下载。
- 输出 Markdown 文件路径镜像源站 URL 路径，目录索引固定名 `index.md`，进度文件固定名 `manifest.json`。

---

## 5. 安全约束（红线）

- **密钥**：LLM API Key 等敏感信息只能来自环境变量或显式配置；**禁止硬编码、禁止写入日志、禁止写入产物或 manifest**。
- **路径安全**：所有由远程 URL 推导出的落盘路径必须经规范化校验，**禁止目录穿越**（不得逃出 `--out` 目录）。
- **网络礼貌**：默认遵守速率限制与并发上限；不绕过登录 / 付费墙 / 验证码；尊重显式的抓取边界（前缀、最大页数）。
- **不执行远端代码**：抓取到的脚本内容仅作文本处理，绝不执行。

---

## 6. 错误处理与可观测性

- 库内部错误用 `thiserror` 定义类型；应用层用 `anyhow` 聚合；**库路径禁止 `unwrap()`/`expect()`/`panic!` 处理可恢复错误**。
- 单页失败不得中断整体（失败隔离），须记录并继续；最终汇总失败清单。
- 日志用 `tracing`，至少可回答 SDD 六维度：**当前目标、当前步骤、所用工具/引擎、失败原因、是否触发重试/降级、本轮耗时**。

---

## 7. 测试要求

- 每个核心模块（discover / extract / convert / assets / writer）须有单元测试。
- 解析与转换类逻辑用本地 fixture（保存的 HTML）测试，**不依赖真实网络**。
- 涉及网络的测试须可跳过 / 标注，不进默认 `cargo test` 阻塞路径。
- 交付前必须通过 `cargo fmt --check`、`cargo clippy -D warnings`、`cargo test`。

---

## 8. 变更治理

- 任何功能变更**先改 `spec.md`，再改代码**（防"规格腐烂"）。
- 微小且不影响其他模块的变更（如修措辞、改默认值）可直接改，不必走全流程（防"规格官僚化"）。
- Spec 与代码同仓库版本管理，一并提交。
- Validate 阶段（自动化测试 + 人工 Review）不可省略，Spec 不替代 Code Review。
