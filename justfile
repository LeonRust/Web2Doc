# Web2Doc 开发任务（constitution §7 交付门禁）

# 默认：列出可用任务
default:
    @just --list

# 门禁：格式 + lint + 测试
check: fmt-check lint test

# 格式化
fmt:
    cargo fmt

# 校验格式
fmt-check:
    cargo fmt --check

# Lint（仅本地包，-D warnings — N-8）
lint:
    cargo clippy --all-targets -- -D warnings

# 测试
test:
    cargo test

# 构建
build:
    cargo build

# 运行（透传参数：just run https://example.com/docs/ --out ./out）
run *ARGS:
    cargo run -- {{ ARGS }}
