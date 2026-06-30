#!/usr/bin/env bash
# Web2Doc Agent Skill 一键安装。
#
# 用法：
#   curl -fsSL https://raw.githubusercontent.com/LeonRust/Web2Doc/main/scripts/install-skill.sh | bash
#   curl -fsSL .../install-skill.sh | bash -s -- ~/.config/opencode/skills   # 指定 skills 父目录
#
# 默认装到 ~/.claude/skills/web2doc/——该位置会被 Claude Code 与 opencode 同时自动发现。
set -euo pipefail

RAW_BASE="${WEB2DOC_RAW_BASE:-https://raw.githubusercontent.com/LeonRust/Web2Doc/main}"

# skills 父目录：位置参数 > 环境变量 WEB2DOC_SKILL_DIR > 默认 ~/.claude/skills
parent="${1:-${WEB2DOC_SKILL_DIR:-$HOME/.claude/skills}}"
parent="${parent/#\~/$HOME}"
dest="$parent/web2doc"

echo "安装 web2doc skill 到 $dest"
mkdir -p "$dest"
if command -v curl >/dev/null 2>&1; then
  curl -fsSL "$RAW_BASE/skills/web2doc/SKILL.md" -o "$dest/SKILL.md"
elif command -v wget >/dev/null 2>&1; then
  wget -qO "$dest/SKILL.md" "$RAW_BASE/skills/web2doc/SKILL.md"
else
  echo "错误：需要 curl 或 wget" >&2
  exit 1
fi
echo "完成：已写入 $dest/SKILL.md"

if command -v web2doc >/dev/null 2>&1; then
  echo "web2doc 已在 PATH。"
else
  echo "提示：web2doc 尚未安装。装预编译二进制（无需 Rust）："
  echo "  curl -fsSL $RAW_BASE/scripts/install.sh | bash"
  echo "（skill 首次使用时也会自动执行同样的安装。）"
fi
echo "请重启你的 AI 编码工具（opencode / Claude Code）以加载该 skill。"
