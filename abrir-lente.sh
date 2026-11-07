#!/bin/sh
# Abre Lente directamente en tu sistema (con acceso a la cámara).
# Sólo compila si falta el programa o el código cambió; si no, arranca al instante.
# Uso: doble clic -> "Ejecutar", o en una terminal: sh abrir-lente.sh
cd "$(dirname "$0")" || exit 1
BIN=target/release/lente
needs_build() {
    [ ! -x "$BIN" ] && return 0
    [ -n "$(find src i18n Cargo.toml Cargo.lock -newer "$BIN" -print -quit 2>/dev/null)" ]
}
if needs_build && command -v cargo >/dev/null 2>&1; then
    echo "Compilando Lente (sólo la primera vez o tras cambios)…"
    cargo build --release -q || exit 1
fi
[ -x "$BIN" ] && exec "$BIN" "$@"
exec target/debug/lente "$@"
