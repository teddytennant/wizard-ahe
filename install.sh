#!/usr/bin/env bash
# wizard-ahe installer: builds the `ahe` CLI, puts it on PATH, and hands off
# to `ahe setup` (interactive: endpoint URL, model, API key, wizard install).
#
# One-liner:
#   bash <(curl -fsSL https://raw.githubusercontent.com/teddytennant/wizard-ahe/main/install.sh)
#
# From an existing checkout:  ./install.sh
# Flags: --no-setup   install the CLI only, skip the interactive setup
set -euo pipefail

REPO_SLUG="teddytennant/wizard-ahe"
CLONE_DIR="${AHE_HOME:-$HOME/.local/share/ahe/wizard-ahe}"
BIN_DIR="$HOME/.local/bin"
RUN_SETUP=1
[ "${1:-}" = "--no-setup" ] && RUN_SETUP=0

say() { printf '\033[1m[ahe install]\033[0m %s\n' "$*"; }
die() { printf '\033[31m[ahe install] %s\033[0m\n' "$*" >&2; exit 1; }

need() { command -v "$1" >/dev/null 2>&1 || die "missing dependency: $1 ($2)"; }
need git "https://git-scm.com"
need docker "the loop runs task containers"
need uv "https://docs.astral.sh/uv/"
command -v cargo >/dev/null 2>&1 || command -v nix-shell >/dev/null 2>&1 \
  || die "need cargo (rustup) or nix-shell to build the CLI"

# 1. Get the lab checkout (skip when running from inside one).
if [ -f evolve.py ] && [ -d agents/wizard_harness ]; then
  LAB_DIR="$(pwd -P)"
  say "using existing checkout: $LAB_DIR"
else
  if [ -d "$CLONE_DIR/.git" ]; then
    say "updating $CLONE_DIR"
    git -C "$CLONE_DIR" pull --ff-only
  else
    say "cloning $REPO_SLUG -> $CLONE_DIR"
    mkdir -p "$(dirname "$CLONE_DIR")"
    git clone "https://github.com/$REPO_SLUG.git" "$CLONE_DIR"
  fi
  LAB_DIR="$CLONE_DIR"
fi

# 2. Build the CLI (nix-shell provides cargo when the host lacks rustup).
say "building the ahe CLI"
if command -v cargo >/dev/null 2>&1; then
  (cd "$LAB_DIR/cli" && cargo build --release)
else
  nix-shell -p cargo rustc --run "cd '$LAB_DIR/cli' && cargo build --release"
fi

# 3. Install to PATH.
mkdir -p "$BIN_DIR"
install -m 755 "$LAB_DIR/cli/target/release/ahe" "$BIN_DIR/ahe"
say "installed $BIN_DIR/ahe"

case ":$PATH:" in
  *":$BIN_DIR:"*) ;;
  *)
    say "adding $BIN_DIR to PATH in your shell config"
    for rc in "$HOME/.bashrc" "$HOME/.zshrc"; do
      [ -f "$rc" ] && ! grep -q '\.local/bin' "$rc" \
        && printf '\nexport PATH="$HOME/.local/bin:$PATH"\n' >> "$rc"
    done
    fish_conf="$HOME/.config/fish/config.fish"
    [ -f "$fish_conf" ] && ! grep -q '\.local/bin' "$fish_conf" \
      && printf '\nfish_add_path -g ~/.local/bin\n' >> "$fish_conf"
    export PATH="$BIN_DIR:$PATH"
    ;;
esac

# 4. Record the lab location so `ahe` works from anywhere, then hand off.
if [ "$RUN_SETUP" = 1 ]; then
  say "starting interactive setup (endpoint, model, key, wizard build)"
  (cd "$LAB_DIR" && "$BIN_DIR/ahe" setup)
else
  say "done — run 'ahe setup' inside $LAB_DIR to finish wiring"
fi
