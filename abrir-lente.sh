#!/bin/sh
# Opens Lente directly on your system (with access to the camera).
# Builds only if the program is missing or the code changed; otherwise it starts at once.
# Usage: double-click -> "Run", or in a terminal: sh abrir-lente.sh
cd "$(dirname "$0")" || exit 1
BIN=target/release/lente
needs_build() {
    [ ! -x "$BIN" ] && return 0
    [ -n "$(find src i18n Cargo.toml Cargo.lock -newer "$BIN" -print -quit 2>/dev/null)" ]
}
if needs_build && command -v cargo >/dev/null 2>&1; then
    echo "Building Lente (only the first time or after changes)…"
    cargo build --release -q || exit 1
fi
[ -x "$BIN" ] && exec "$BIN" "$@"
exec target/debug/lente "$@"
