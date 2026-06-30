# Web2Doc 二进制安装（Windows / PowerShell）：从 GitHub Releases 下载预编译二进制（无需 Rust）。
#
# 用法：
#   irm https://raw.githubusercontent.com/LeonRust/Web2Doc/main/scripts/install.ps1 | iex
#   指定目录：$env:WEB2DOC_BIN_DIR="C:\tools"; irm .../install.ps1 | iex

$ErrorActionPreference = "Stop"
# Windows PowerShell 5.1 默认可能不启用 TLS1.2，而 GitHub 需要它
[Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12

$Repo = "LeonRust/Web2Doc"
$BinDir = if ($env:WEB2DOC_BIN_DIR) { $env:WEB2DOC_BIN_DIR } else { Join-Path $env:LOCALAPPDATA "web2doc\bin" }
$Base = if ($env:WEB2DOC_DL_BASE) { $env:WEB2DOC_DL_BASE } else { "https://github.com/$Repo/releases/latest/download" }

# 选 Windows 目标架构（X64 / Arm64 均有预编译版）
$osArch = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture
if ("$osArch" -eq "Arm64") { $tgt = "aarch64-pc-windows-msvc" } else { $tgt = "x86_64-pc-windows-msvc" }
$asset = "web2doc-$tgt.zip"
$url = "$Base/$asset"

$tmp = Join-Path $env:TEMP ("web2doc-" + [System.Guid]::NewGuid().ToString())
New-Item -ItemType Directory -Path $tmp -Force | Out-Null
try {
    $zip = Join-Path $tmp $asset
    Write-Host "下载 $asset ..."
    try {
        Invoke-WebRequest -Uri $url -OutFile $zip -UseBasicParsing
    } catch {
        throw "下载失败（可能尚未发布该平台版本）。可改用：cargo install --git https://github.com/$Repo （需 Rust 1.85+）"
    }

    # SHA256 校验：能下到 .sha256 就比对，不一致则中止；下不到则跳过
    $haveSha = $false
    try {
        Invoke-WebRequest -Uri "$url.sha256" -OutFile "$zip.sha256" -UseBasicParsing
        $haveSha = $true
    } catch { Write-Host "（无 .sha256，跳过校验）" }
    if ($haveSha) {
        $expected = (((Get-Content "$zip.sha256" -Raw).Trim() -split "\s+")[0]).ToLower()
        $actual = (Get-FileHash $zip -Algorithm SHA256).Hash.ToLower()
        if ($expected -ne $actual) { throw "SHA256 校验失败！expected=$expected actual=$actual" }
        Write-Host "SHA256 校验通过"
    }

    Expand-Archive -Path $zip -DestinationPath $tmp -Force
    $exe = Get-ChildItem -Path $tmp -Recurse -Filter "web2doc.exe" | Select-Object -First 1
    if (-not $exe) { throw "归档中未找到 web2doc.exe" }

    New-Item -ItemType Directory -Path $BinDir -Force | Out-Null
    Copy-Item -Path $exe.FullName -Destination (Join-Path $BinDir "web2doc.exe") -Force
    Write-Host "已安装到 $BinDir\web2doc.exe"

    # 写入用户 PATH（持久）+ 当前进程 PATH（即时可用）
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if (-not $userPath) { $userPath = "" }
    if ($userPath -notlike "*$BinDir*") {
        [Environment]::SetEnvironmentVariable("Path", ($userPath.TrimEnd(";") + ";" + $BinDir), "User")
        Write-Host "已把 $BinDir 加入用户 PATH（新开终端生效）。"
    }
    $env:Path = $env:Path + ";" + $BinDir
} finally {
    Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
}
