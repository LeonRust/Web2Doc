# Web2Doc Agent Skill 一键安装（Windows / PowerShell）。
#
# 用法：
#   irm https://raw.githubusercontent.com/LeonRust/Web2Doc/main/scripts/install-skill.ps1 | iex
#   指定 skills 父目录：$env:WEB2DOC_SKILL_DIR="$HOME\.config\opencode\skills"; irm .../install-skill.ps1 | iex
#
# 默认装到 ~\.claude\skills\web2doc\——该位置会被 Claude Code 与 opencode 同时自动发现。

$ErrorActionPreference = "Stop"
[Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12

$RawBase = if ($env:WEB2DOC_RAW_BASE) { $env:WEB2DOC_RAW_BASE } else { "https://raw.githubusercontent.com/LeonRust/Web2Doc/main" }

# skills 父目录：脚本参数 > 环境变量 WEB2DOC_SKILL_DIR > 默认 ~\.claude\skills
$parent = $env:WEB2DOC_SKILL_DIR
if ($args.Count -ge 1) { $parent = $args[0] }
if (-not $parent) { $parent = Join-Path $HOME ".claude\skills" }
$dest = Join-Path $parent "web2doc"

Write-Host "安装 web2doc skill 到 $dest"
New-Item -ItemType Directory -Path $dest -Force | Out-Null
Invoke-WebRequest -Uri "$RawBase/skills/web2doc/SKILL.md" -OutFile (Join-Path $dest "SKILL.md") -UseBasicParsing
Write-Host "完成：已写入 $dest\SKILL.md"

if (Get-Command web2doc -ErrorAction SilentlyContinue) {
    Write-Host "web2doc 已在 PATH。"
} else {
    Write-Host "提示：web2doc 尚未安装。装预编译二进制（无需 Rust）："
    Write-Host "  irm $RawBase/scripts/install.ps1 | iex"
    Write-Host "（skill 首次使用时也会自动执行同样的安装。）"
}
Write-Host "请重启你的 AI 编码工具（opencode / Claude Code）以加载该 skill。"
