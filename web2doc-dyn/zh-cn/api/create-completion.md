# FIM 补全（Beta） | DeepSeek API Docs

```
curl -L -X POST 'https://api.deepseek.com/beta/completions' \
-H 'Content-Type: application/json' \
-H 'Accept: application/json' \
-H 'Authorization: Bearer <TOKEN>' \
--data-raw '{
  "model": "deepseek-v4-pro",
  "prompt": "Once upon a time, ",
  "echo": false,
  "logprobs": 0,
  "max_tokens": 1024,
  "stop": null,
  "stream": false,
  "stream_options": null,
  "suffix": null,
  "temperature": 1,
  "top_p": 1
}'
```