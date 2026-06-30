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

# 安装为 AI Coding Skill（默认 opencode 全局；可传目标 skills 目录）
#   just install-skill                       -> ~/.config/opencode/skills
#   just install-skill ~/.claude/skills      -> Claude Code 全局
#   just install-skill .opencode/skills      -> 当前项目(opencode)
install-skill dest="~/.config/opencode/skills":
    #!/usr/bin/env bash
    set -euo pipefail
    dest="{{ dest }}"
    dest="${dest/#\~/$HOME}"
    mkdir -p "$dest/web2doc"
    cp skills/web2doc/SKILL.md "$dest/web2doc/SKILL.md"
    echo "已安装 web2doc skill 到 $dest/web2doc/ —— 重启 AI 工具后生效"
