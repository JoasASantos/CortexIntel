#!/usr/bin/env bash
# CortexIntel install/setup — builds the release binary, scaffolds demo data,
# and (optionally) installs `cortex` onto your PATH.
set -euo pipefail

BOLD=$'\033[1m'; DIM=$'\033[2m'; GRN=$'\033[32m'; CYN=$'\033[36m'; RST=$'\033[0m'
say() { printf "%s\n" "$*"; }

say "${BOLD}${CYN}CortexIntel${RST} — install"
say ""

# 1) Rust toolchain
if ! command -v cargo >/dev/null 2>&1; then
  say "${BOLD}Rust not found.${RST} Install it first: https://rustup.rs"
  say "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
  exit 1
fi
say "· cargo $(cargo --version | awk '{print $2}')"

# 2) Build
say "· building release binary (cargo build --release)…"
cargo build --release
BIN="$(pwd)/target/release/cortex"
say "  ${GRN}built${RST} → ${BIN}"

# 3) Scaffold demo data (idempotent)
if [ ! -d "./cortex-demo" ]; then
  say "· scaffolding demo dataset (./cortex-demo)…"
  "$BIN" init --dir ./cortex-demo >/dev/null 2>&1 || true
fi

# 4) Optional: install onto PATH
DEST="${CORTEX_INSTALL_DIR:-$HOME/.local/bin}"
if [ "${1:-}" = "--install" ] || [ "${CORTEX_INSTALL:-}" = "1" ]; then
  mkdir -p "$DEST"
  cp "$BIN" "$DEST/cortex"
  say "· installed to ${GRN}${DEST}/cortex${RST}"
  case ":$PATH:" in *":$DEST:"*) ;; *) say "  ${DIM}add to PATH: export PATH=\"$DEST:\$PATH\"${RST}";; esac
fi

say ""
say "${BOLD}Ready.${RST} Try it:"
say "  ${CYN}${BIN} run -i ./cortex-demo/reports.csv --domain fraud --offline${RST}   # deterministic, no cost"
say "  ${CYN}${BIN} serve --open${RST}                                              # the GUI in your browser"
say ""
say "${DIM}Docs: README.md · docs/USAGE.md · docs/PLUGINS.md · docs/ROADMAP.md${RST}"
