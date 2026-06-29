# 对话补全 | DeepSeek API Docs

POST

## /chat/completions

根据输入的上下文，来让模型补全对话内容。

## Request

*   application/json

### Body

**required**

**messages**

object\[\]required

**Possible values:** `>= 1`

对话的消息列表。

*   Array \[

oneOf

*   System message
*   User message
*   Assistant message
*   Tool message

**content** stringrequired

system 消息的内容。

**role** stringrequired

**Possible values:** \[`system`\]

该消息的发起角色，其值为 `system`。

**name** string

可以选填的参与者的名称，为模型提供信息以区分相同角色的参与者。

*   \]

**model** stringrequired

**Possible values:** \[`deepseek-v4-flash`, `deepseek-v4-pro`\]

使用的模型的 ID。

**thinking**

objectnullable

控制思考模式与非思考模式的转换

**type** string

**Possible values:** \[`enabled`, `disabled`\]

**Default value:** `enabled`

如果设为 `enabled`，则使用思考模式。如果设为 `disabled`，则使用非思考模式

**reasoning\_effort** string

**Possible values:** \[`high`, `max`\]

控制模型的推理强度。对普通请求，默认为 `high`。对一些复杂 Agent 类请求（如 Claude Code、OpenCode），自动设置为 `max`。出于兼容考虑 `low`、`medium` 会映射为 `high`, `xhigh` 会映射为 `max`。

**max\_tokens** integernullable

限制一次请求中模型生成 completion 的最大 token 数。输入 token 和输出 token 的总长度受模型的上下文长度的限制。取值范围与默认值详见[文档](https://api-docs.deepseek.com/zh-cn/quick_start/pricing)。

**response\_format**

objectnullable

一个 object，指定模型必须输出的格式。

设置为 { "type": "json\_object" } 以启用 JSON 模式，该模式保证模型生成的消息是有效的 JSON。

**注意:** 使用 JSON 模式时，你还必须通过系统或用户消息指示模型生成 JSON。否则，模型可能会生成不断的空白字符，直到生成达到令牌限制，从而导致请求长时间运行并显得“卡住”。此外，如果 finish\_reason="length"，这表示生成超过了 max\_tokens 或对话超过了最大上下文长度，消息内容可能会被部分截断。

**type** string

**Possible values:** \[`text`, `json_object`\]

**Default value:** `text`

Must be one of `text` or `json_object`.

**stop**

object

**nullable**

一个 string 或最多包含 16 个 string 的 list，在遇到这些词时，API 将停止生成更多的 token。

oneOf

*   MOD1
*   MOD2

string

**stream** booleannullable

如果设置为 True，将会以 SSE（server-sent events）的形式以流式发送消息增量。消息流以 `data: [DONE]` 结尾。

**stream\_options**

objectnullable

流式输出相关选项。只有在 `stream` 参数为 `true` 时，才可设置此参数。

**include\_usage** boolean

如果设置为 true，在流式消息最后的 `data: [DONE]` 之前将会传输一个额外的块。此块上的 usage 字段显示整个请求的 token 使用统计信息，而 choices 字段将始终是一个空数组。所有其他块也将包含一个 usage 字段，但其值为 null。

**temperature** numbernullable

**Possible values:** `<= 2`

**Default value:** `1`

采样温度，介于 0 和 2 之间。更高的值，如 0.8，会使输出更随机，而更低的值，如 0.2，会使其更加集中和确定。 我们通常建议可以更改这个值或者更改 `top_p`，但不建议同时对两者进行修改。

**top\_p** numbernullable

**Possible values:** `<= 1`

**Default value:** `1`

作为调节采样温度的替代方案，模型会考虑前 `top_p` 概率的 token 的结果。所以 0.1 就意味着只有包括在最高 10% 概率中的 token 会被考虑。 我们通常建议修改这个值或者更改 `temperature`，但不建议同时对两者进行修改。

**tools**

object\[\]nullable

模型可能会调用的 tool 的列表。目前，仅支持 function 作为工具。使用此参数来提供以 JSON 作为输入参数的 function 列表。最多支持 128 个 function。

*   Array \[

**type** stringrequired

**Possible values:** \[`function`\]

tool 的类型。目前仅支持 function。

**function**

objectrequired

**description** string

function 的功能描述，供模型理解何时以及如何调用该 function。

**name** stringrequired

要调用的 function 名称。必须由 a-z、A-Z、0-9 字符组成，或包含下划线和连字符，最大长度为 64 个字符。

**parameters**

object

**strict** boolean

**Default value:** `false`

如果设置为 true，API 将在函数调用中使用 strict 模式，以确保输出始终符合函数的 JSON schema 定义。该功能为 Beta 功能，详细使用方式请参阅[Tool Calls 指南](https://api-docs.deepseek.com/zh-cn/guides/tool_calls)

*   \]

**tool\_choice**

object

**nullable**

控制模型调用 tool 的行为。

`none` 意味着模型不会调用任何 tool，而是生成一条消息。

`auto` 意味着模型可以选择生成一条消息或调用一个或多个 tool。

`required` 意味着模型必须调用一个或多个 tool。

通过 `{"type": "function", "function": {"name": "my_function"}}` 指定特定 tool，会强制模型调用该 tool。

当没有 tool 时，默认值为 `none`。如果有 tool 存在，默认值为 `auto`。

oneOf

*   ChatCompletionToolChoice
*   ChatCompletionNamedToolChoice

string

**Possible values:** \[`none`, `auto`, `required`\]

**logprobs** booleannullable

是否返回所输出 token 的对数概率。如果为 true，则在 `message` 的 `content` 中返回每个输出 token 的对数概率。

**top\_logprobs** integernullable

**Possible values:** `<= 20`

一个介于 0 到 20 之间的整数 N，指定每个输出位置返回输出概率 top N 的 token，且返回这些 token 的对数概率。指定此参数时，logprobs 必须为 true。

**user\_id** nullable

您自定义的 user\_id，可选字符集为 \[a-zA-Z0-9\\-\_\]，最大长度为 512。请不要在 user\_id 中包含用户隐私信息。

*   user\_id 可用于区分您业务侧的用户身份，以帮助我们进行内容安全处理。
*   user\_id 可用于 KVCache 缓存隔离，以进行隐私管理。
*   user\_id 可用于我们对您业务侧用户进行调度隔离。
*   关于 user\_id 参数更详细的描述，请参考[限速与隔离](https://api-docs.deepseek.com/zh-cn/quick_start/rate_limit)

**frequency\_penalty** deprecated

该参数已不再支持。传入该参数将不会产生任何效果。

**presence\_penalty** deprecated

该参数已不再支持。传入该参数将不会产生任何效果。

OK, 返回一个 `chat completion` 对象。

*   application/json

*   Schema
*   Example (from schema)
*   Example

**Schema**

**id** stringrequired

该对话的唯一标识符。

**choices**

object\[\]required

模型生成的 completion 的选择列表。

*   Array \[

**finish\_reason** stringrequired

**Possible values:** \[`stop`, `length`, `content_filter`, `tool_calls`, `insufficient_system_resource`\]

模型停止生成 token 的原因。

`stop`：模型自然停止生成，或遇到 `stop` 序列中列出的字符串。

`length` ：输出长度达到了模型上下文长度限制，或达到了 `max_tokens` 的限制。

`content_filter`：输出内容因触发过滤策略而被过滤。

`insufficient_system_resource`：系统推理资源不足，生成被打断。

**index** integerrequired

该 completion 在模型生成的 completion 的选择列表中的索引。

**message**

objectrequired

模型生成的 completion 消息。

**content** stringnullablerequired

该 completion 的内容。

**reasoning\_content** stringnullable

仅适用于思考模式。内容为 assistant 消息中在最终答案之前的推理内容。

**tool\_calls**

object\[\]

模型生成的 tool 调用，例如 function 调用。

*   Array \[

**id** stringrequired

tool 调用的 ID。

**type** stringrequired

**Possible values:** \[`function`\]

tool 的类型。目前仅支持 `function`。

**function**

objectrequired

模型调用的 function。

**name** stringrequired

模型调用的 function 名。

**arguments** stringrequired

要调用的 function 的参数，由模型生成，格式为 JSON。请注意，模型并不总是生成有效的 JSON，并且可能会臆造出你函数模式中未定义的参数。在调用函数之前，请在代码中验证这些参数。

*   \]

**role** stringrequired

**Possible values:** \[`assistant`\]

生成这条消息的角色。

**logprobs**

objectnullablerequired

该 choice 的对数概率信息。

**content**

object\[\]nullablerequired

一个包含输出 token 对数概率信息的列表。

*   Array \[

**token** stringrequired

输出的 token。

**logprob** numberrequired

该 token 的对数概率。`-9999.0` 代表该 token 的输出概率极小，不在 top 20 最可能输出的 token 中。

**bytes** integer\[\]nullablerequired

一个包含该 token UTF-8 字节表示的整数列表。一般在一个 UTF-8 字符被拆分成多个 token 来表示时有用。如果 token 没有对应的字节表示，则该值为 `null`。

**top\_logprobs**

object\[\]required

一个包含在该输出位置上，输出概率 top N 的 token 的列表，以及它们的对数概率。在罕见情况下，返回的 token 数量可能少于请求参数中指定的 `top_logprobs` 值。

*   Array \[

**token** stringrequired

输出的 token。

**logprob** numberrequired

该 token 的对数概率。`-9999.0` 代表该 token 的输出概率极小，不在 top 20 最可能输出的 token 中。

**bytes** integer\[\]nullablerequired

一个包含该 token UTF-8 字节表示的整数列表。一般在一个 UTF-8 字符被拆分成多个 token 来表示时有用。如果 token 没有对应的字节表示，则该值为 `null`。

*   \]

*   \]

**reasoning\_content**

object\[\]nullable

一个包含输出 token 对数概率信息的列表。

*   Array \[

**token** stringrequired

输出的 token。

**logprob** numberrequired

该 token 的对数概率。`-9999.0` 代表该 token 的输出概率极小，不在 top 20 最可能输出的 token 中。

**bytes** integer\[\]nullablerequired

一个包含该 token UTF-8 字节表示的整数列表。一般在一个 UTF-8 字符被拆分成多个 token 来表示时有用。如果 token 没有对应的字节表示，则该值为 `null`。

**top\_logprobs**

object\[\]required

一个包含在该输出位置上，输出概率 top N 的 token 的列表，以及它们的对数概率。在罕见情况下，返回的 token 数量可能少于请求参数中指定的 `top_logprobs` 值。

*   Array \[

**token** stringrequired

输出的 token。

**logprob** numberrequired

该 token 的对数概率。`-9999.0` 代表该 token 的输出概率极小，不在 top 20 最可能输出的 token 中。

**bytes** integer\[\]nullablerequired

一个包含该 token UTF-8 字节表示的整数列表。一般在一个 UTF-8 字符被拆分成多个 token 来表示时有用。如果 token 没有对应的字节表示，则该值为 `null`。

*   \]

*   \]

*   \]

**created** integerrequired

创建聊天完成时的 Unix 时间戳（以秒为单位）。

**model** stringrequired

生成该 completion 的模型名。

**system\_fingerprint** stringrequired

This fingerprint represents the backend configuration that the model runs with.

**object** stringrequired

**Possible values:** \[`chat.completion`\]

对象的类型, 其值为 `chat.completion`。

**usage**

object

该对话补全请求的用量信息。

**completion\_tokens** integerrequired

模型 completion 产生的 token 数。

**prompt\_tokens** integerrequired

用户 prompt 所包含的 token 数。该值等于 `prompt_cache_hit_tokens + prompt_cache_miss_tokens`

**prompt\_cache\_hit\_tokens** integerrequired

用户 prompt 中，命中上下文缓存的 token 数。

**prompt\_cache\_miss\_tokens** integerrequired

用户 prompt 中，未命中上下文缓存的 token 数。

**total\_tokens** integerrequired

该请求中，所有 token 的数量（prompt + completion）。

**completion\_tokens\_details**

object

completion tokens 的详细信息。

**reasoning\_tokens** integer

推理模型所产生的思维链 token 数量