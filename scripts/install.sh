#!/usr/bin/env bash
# Web2Doc 二进制安装：从 GitHub Releases 下载对应平台的预编译二进制（无需 Rust）。
#
# 用法：
#   curl -fsSL https://raw.githubusercontent.com/LeonRust/Web2Doc/main/scripts/install.sh | bash
#   curl -fsSL .../install.sh | bash -s -- ~/bin    # 指定安装目录（默认 ~/.local/bin）
set -euo pipefail

REPO="LeonRust/Web2Doc"
bin_dir="${1:-${WEB2DOC_BIN_DIR:-$HOME/.local/bin}}"
bin_dir="${bin_dir/#\~/$HOME}"
base="${WEB2DOC_DL_BASE:-https://github.com/${REPO}/releases/latest/download}"

# 探测平台 -> Rust target 三元组（与 release.yml 的附件命名一致）
os="$(uname -s)"
arch="$(uname -m)"
case "$os" in
  Linux) os_part="unknown-linux-gnu" ;;
  Darwin) os_part="apple-darwin" ;;
  *)
    echo "不支持自动安装的系统：$os" >&2
    echo "Windows 请从 https://github.com/$REPO/releases 下载 .zip，或用 cargo install。" >&2
    exit 1
    ;;
esac
case "$arch" in
  x86_64 | amd64) arch_part="x86_64" ;;
  arm64 | aarch64) arch_part="aarch64" ;;
  *)
    echo "不支持的架构：$arch" >&2
    exit 1
    ;;
esac

# 一键脚本默认装 gnu 版（兼容大多数发行版）；静态 musl 版请从 Releases 手动下载。
asset="web2doc-${arch_part}-${os_part}.tar.gz"

dl() { # dl <url> <out>
  if command -v curl >/dev/null 2>&1; then
    curl -fsSL "$1" -o "$2"
  elif command -v wget >/dev/null 2>&1; then
    wget -qO "$2" "$1"
  else
    echo "需要 curl 或 wget" >&2
    return 1
  fi
}

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

echo "下载 $asset ..."
if ! dl "$base/$asset" "$tmp/$asset"; then
  echo "下载失败（可能尚未发布该平台版本）。" >&2
  echo "可改用：cargo install --git https://github.com/$REPO （需 Rust 1.85+）" >&2
  exit 1
fi

# SHA256 校验（发布了 .sha256 且本机有校验工具时执行；不匹配则中止）
if dl "$base/$asset.sha256" "$tmp/$asset.sha256" 2>/dev/null; then
  if command -v sha256sum >/dev/null 2>&1; then
    sum="sha256sum"
  elif command -v shasum >/dev/null 2>&1; then
    sum="shasum -a 256"
  else
    sum=""
  fi
  if [ -n "$sum" ]; then
    expected="$(cut -d' ' -f1 "$tmp/$asset.sha256")"
    actual="$($sum "$tmp/$asset" | cut -d' ' -f1)"
    if [ "$expected" != "$actual" ]; then
      echo "SHA256 校验失败！expected=$expected actual=$actual" >&2
      exit 1
    fi
    echo "SHA256 校验通过"
  fi
fi

tar -xzf "$tmp/$asset" -C "$tmp"
binpath="$(find "$tmp" -type f -name web2doc | head -n1)"
[ -n "$binpath" ] || {
  echo "归档中未找到 web2doc" >&2
  exit 1
}
mkdir -p "$bin_dir"
install -m 0755 "$binpath" "$bin_dir/web2doc"
echo "已安装到 $bin_dir/web2doc"

case ":$PATH:" in
  *":$bin_dir:"*) ;;
  *)
    echo "注意：$bin_dir 不在 PATH 中，请加入后重开终端，例如：" >&2
    echo "  echo 'export PATH=\"$bin_dir:\$PATH\"' >> ~/.zshrc" >&2
    ;;
esac
"$bin_dir/web2doc" --version 2>/dev/null || true
