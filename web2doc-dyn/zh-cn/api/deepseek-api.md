# Deepseek API | DeepSeek API Docs

!function(){function t(t){document.documentElement.setAttribute("data-theme",t)}var e=function(){try{return new URLSearchParams(window.location.search).get("docusaurus-theme")}catch(t){}}()||function(){try{return localStorage.getItem("theme")}catch(t){}}();t(null!==e?e:"light")}(),function(){try{const c=new URLSearchParams(window.location.search).entries();for(var\[t,e\]of c)if(t.startsWith("docusaurus-data-")){var a=t.replace("docusaurus-data-","data-");document.documentElement.setAttribute(a,e)}}catch(t){}}()

[跳到主要内容](#__docusaurus_skipToContent_fallback)

[![DeepSeek API 文档 Logo](../../assets/87dfb459046c3805.png)

**DeepSeek API 文档**](/zh-cn/)

[中文（中国）](#)

*   [English](/api/deepseek-api)
*   [中文（中国）](/zh-cn/api/deepseek-api)

[DeepSeek Platform](https://platform.deepseek.com/)

[![DeepSeek API 文档 Logo](../../assets/87dfb459046c3805.png)

**DeepSeek API 文档**](/zh-cn/)

*   选择语言
*   [DeepSeek Platform](https://platform.deepseek.com/)

← 回到主菜单

*   [快速开始](#)

    *   [首次调用 API](/zh-cn/)
    *   [模型 & 价格](/zh-cn/quick_start/pricing)
    *   [Token 用量计算](/zh-cn/quick_start/token_usage)
    *   [限速与隔离](/zh-cn/quick_start/rate_limit)
    *   [错误码](/zh-cn/quick_start/error_codes)
    *   [接入 Agent 工具](#)
*   [API 指南](#)

    *   [思考模式](/zh-cn/guides/thinking_mode)
    *   [多轮对话](/zh-cn/guides/multi_round_chat)
    *   [对话前缀续写（Beta）](/zh-cn/guides/chat_prefix_completion)
    *   [FIM 补全（Beta）](/zh-cn/guides/fim_completion)
    *   [JSON Output](/zh-cn/guides/json_mode)
    *   [Tool Calls](/zh-cn/guides/tool_calls)
    *   [上下文硬盘缓存](/zh-cn/guides/kv_cache)
    *   [Anthropic API](/zh-cn/guides/anthropic_api)
*   [API 文档](#)

    *   [基本信息](/zh-cn/api/deepseek-api)
    *   [对话（Chat）](#)

    *   [补全（Completions）](#)

    *   [模型（Model）](#)

    *   [其它](#)
*   [新闻](#)

*   [其它资源](#)

*   [常见问题](/zh-cn/faq)
*   [更新日志](/zh-cn/updates)

*   [](/zh-cn/)
*   API 文档
*   基本信息

Version: 1.0.0

# Deepseek API

使用 DeepSeek API 之前，请先 [创建 API 密钥](https://platform.deepseek.com/api_keys)。

## Authentication

*   HTTP: Bearer Auth

| Security Scheme Type: |
| --------------------- |
| bearer                |

### Contact

DeepSeek 技术支持: [api-service@deepseek.com](mailto:api-service@deepseek.com)

### Terms of Service

[](https://cdn.deepseek.com/policies/zh-CN/deepseek-open-platform-terms-of-service.html)

[](https://cdn.deepseek.com/policies/zh-CN/deepseek-open-platform-terms-of-service.html)[https://cdn.deepseek.com/policies/zh-CN/deepseek-open-platform-terms-of-service.html](https://cdn.deepseek.com/policies/zh-CN/deepseek-open-platform-terms-of-service.html)

### License

[MIT](https://opensource.org/license/mit/)

[上一页

Anthropic API](/zh-cn/guides/anthropic_api)

[下一页

对话补全](/zh-cn/api/create-chat-completion)

微信公众号

*   ![WeChat QRcode](../../assets/3bda99da0a78513a.jpg)

社区

*   [邮箱](mailto:api-service@deepseek.com)
*   [Discord](https://discord.gg/Tc7c45Zzu5)
*   [Twitter](https://twitter.com/deepseek_ai)

更多

*   [GitHub](https://github.com/deepseek-ai)

Copyright © 2026 DeepSeek, Inc.