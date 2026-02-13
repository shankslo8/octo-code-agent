#!/usr/bin/env bash
set -euo pipefail

REPO="https://github.com/user/octo-code-agent"
BINARY="octo-code"

echo ""
echo "  🐙 OctoCode Agent Installer"
echo ""

# --- Detect OS & Architecture ---
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  Linux)  OS_TAG="linux" ;;
  Darwin) OS_TAG="macos" ;;
  *)      echo "  Unsupported OS: $OS"; exit 1 ;;
esac

case "$ARCH" in
  x86_64|amd64)  ARCH_TAG="x86_64" ;;
  aarch64|arm64) ARCH_TAG="aarch64" ;;
  *)             echo "  Unsupported arch: $ARCH"; exit 1 ;;
esac

echo "  Platform: ${OS_TAG}-${ARCH_TAG}"

# --- Try downloading pre-built binary ---
try_download() {
  local tag="latest"
  local url="${REPO}/releases/${tag}/download/${BINARY}-${OS_TAG}-${ARCH_TAG}"

  if command -v curl &>/dev/null; then
    if curl -fsSL "$url" -o "/tmp/${BINARY}" 2>/dev/null; then
      return 0
    fi
  elif command -v wget &>/dev/null; then
    if wget -q "$url" -O "/tmp/${BINARY}" 2>/dev/null; then
      return 0
    fi
  fi
  return 1
}

install_binary() {
  local src="$1"
  chmod +x "$src"

  # Try /usr/local/bin, fall back to ~/.local/bin
  if [ -w /usr/local/bin ]; then
    mv "$src" /usr/local/bin/${BINARY}
    echo "  Installed to /usr/local/bin/${BINARY}"
  else
    mkdir -p "$HOME/.local/bin"
    mv "$src" "$HOME/.local/bin/${BINARY}"
    echo "  Installed to ~/.local/bin/${BINARY}"
    echo ""
    echo "  Make sure ~/.local/bin is in your PATH:"
    echo "    export PATH=\"\$HOME/.local/bin:\$PATH\""
  fi
}

# --- Method 1: Pre-built binary ---
echo ""
echo "  [1/3] Trying pre-built binary..."
if try_download; then
  install_binary "/tmp/${BINARY}"
  echo ""
  echo "  ✓ Done! Run: ${BINARY}"
  exit 0
fi
echo "  No pre-built binary found, trying other methods..."

# --- Method 2: Nix ---
echo "  [2/3] Checking for Nix..."
if command -v nix &>/dev/null; then
  echo "  Nix found. Building with nix..."
  echo ""
  echo "  You can run directly with:"
  echo "    nix run github:user/octo-code-agent"
  echo ""
  echo "  Or install to profile:"
  echo "    nix profile install github:user/octo-code-agent"
  echo ""
  read -p "  Install to Nix profile now? [Y/n] " answer
  answer="${answer:-Y}"
  if [[ "$answer" =~ ^[Yy]$ ]]; then
    nix profile install github:user/octo-code-agent
    echo "  ✓ Done!"
    exit 0
  fi
fi

# --- Method 3: Cargo (build from source) ---
echo "  [3/3] Building from source with Cargo..."

if ! command -v cargo &>/dev/null; then
  echo ""
  echo "  Cargo not found. Installing Rust toolchain..."
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
  source "$HOME/.cargo/env"
fi

echo "  Cloning and building..."
TMPDIR="$(mktemp -d)"
git clone --depth 1 "$REPO" "$TMPDIR/octo-code-agent"
cargo install --path "$TMPDIR/octo-code-agent/crates/octo-cli"
rm -rf "$TMPDIR"

echo ""
echo "  ✓ Done! Run: ${BINARY}"
echo ""
echo "  Set your API key:"
echo "    export ATLAS_API_KEY=your-key-here"
echo ""
