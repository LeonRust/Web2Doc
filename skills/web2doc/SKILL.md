---
name: web2doc
description: 用 web2doc 命令行把在线文档站抓成本地 Markdown，作为编码上下文喂给自己。Use when you need current or official documentation for a library, framework, SDK, or HTTP API whose usage is unfamiliar or whose knowledge may be outdated, or when the user provides a documentation site URL, or before writing non-trivial code against a fast-moving dependency. 触发词：最新文档、官方文档、抓文档、爬文档、docs、API 参考、SDK 文档、不确定某库怎么用、给你个文档链接、喂上下文、web2doc。
---

# web2doc — 抓官方文档喂给自己

当你对某个库 / 框架 / SDK / API 的用法没把握，或担心训练知识过时时，用 `web2doc`
把它的**官方文档抓成本地 Markdown**，再读进来作为权威上下文，避免凭记忆编 API。

## 1. 确认工具可用

```bash
command -v web2doc >/dev/null 2>&1 && echo "ok" || echo "未安装"
```

未安装时（需 Rust 1.85+）：

```bash
cargo install --git https://github.com/LeonRust/Web2Doc   # 或在仓库内：cargo install --path .
```

## 2. 抓取

```bash
# 通用（静态站，无需 Chrome）；--bundle 产出单文件，最适合投喂
web2doc <文档站URL> --bundle --max-pages 80 -o ./.web2doc/<slug>
```

按需追加：

- SPA / 客户端渲染站：`--mode dynamic`（需本机 Chrome）
- 只抓某一块：`--prefix /api/`（限定路径前缀，省 token）
- API 参考（多语言 tab / 嵌套表格）：`--format html`
- 控制规模 / token：`--max-pages N`
- 站点需代理：`--proxy http://host:port`（或设环境变量 `HTTPS_PROXY`）

## 3. 读取产物

- **首选单文件**：`./.web2doc/<slug>/_bundle.md`（全文合并，直接读入上下文）
- 文档很大时：先读 `./.web2doc/<slug>/index.md`（导航索引），再按需读具体页，或对该目录 grep 关键 API

## 4. 续传 / 刷新 / 复用

- 重跑同一命令自动续传（跳过已抓页）；`--fresh` 强制重抓
- 产物可跨会话复用——下次同库直接读已有 `_bundle.md`，不必重抓
- 建议在项目 `.gitignore` 加入 `.web2doc/`

## 5. 注意

- LLM 规则分析可选：设了 `LLM_API_KEY`（或 `.env` / `~/.config/web2doc/config.toml`）时正文识别更准；不设也能用内置规则抓
- 默认尊重 `robots.txt`；用 `--max-pages` 把规模和 token 控制在合理范围
- 抓完后**优先引用产物里的真实 API 签名 / 示例**，而不是凭记忆作答
