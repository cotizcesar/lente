#!/bin/sh
# Instala Lente desde la terminal.
#
#   curl -fsSL https://raw.githubusercontent.com/cotizcesar/lente/main/install.sh | sh
#   curl -fsSL https://raw.githubusercontent.com/cotizcesar/lente/main/install.sh | sh -s -- --user
#
# Opciones:
#   --user            sólo para tu usuario, en ~/.local, sin contraseña (la regla USB aparte)
#   --version X.Y.Z   una versión concreta (por defecto la última)
#   --uninstall       quitar Lente (usa --user si lo instalaste así)
#
# También sirve desde el tarball descomprimido (sh install.sh) o desde el código fuente
# (compila con cargo). Instala el programa, su icono y entrada de menú, y la regla de udev
# que da a tu usuario el acceso USB a las cámaras Elgato.
set -eu

REPO="${LENTE_REPO:-cotizcesar/lente}"
ASSET="lente-linux-x86_64.tar.gz"
MODE=system
ACTION=install
VERSION=""
while [ $# -gt 0 ]; do
    case "$1" in
        --user) MODE=user ;;
        --system) MODE=system ;;
        --uninstall) ACTION=uninstall ;;
        --version) VERSION="${2:?falta el número de versión}"; shift ;;
        --version=*) VERSION="${1#--version=}" ;;
        -h|--help) sed -n '2,16p' "$0" 2>/dev/null || true; exit 0 ;;
        *) echo "Opción desconocida: $1" >&2; exit 2 ;;
    esac
    shift
done

if [ "$MODE" = user ]; then
    PREFIX="$HOME/.local"
    UDEV="-"
else
    PREFIX=/usr/local
    UDEV=/etc/udev/rules.d
fi

say() { printf '\033[1m%s\033[0m\n' "$*"; }

# Ejecuta como administrador cuando hace falta.
as_root() {
    if [ "$MODE" = user ] || [ "$(id -u)" = 0 ]; then
        "$@"
    elif command -v sudo >/dev/null 2>&1; then
        sudo "$@"
    elif command -v doas >/dev/null 2>&1; then
        doas "$@"
    else
        echo "Hace falta sudo o doas para instalar en $PREFIX (o usa --user)." >&2
        exit 1
    fi
}

refresh_system() {
    if [ "$UDEV" != "-" ] && command -v udevadm >/dev/null 2>&1; then
        as_root udevadm control --reload-rules || true
        as_root udevadm trigger --subsystem-match=usb --attr-match=idVendor=0fd9 || true
    fi
    command -v update-desktop-database >/dev/null 2>&1 && as_root update-desktop-database -q "$PREFIX/share/applications" 2>/dev/null || true
    command -v gtk-update-icon-cache >/dev/null 2>&1 && as_root gtk-update-icon-cache -qtf "$PREFIX/share/icons/hicolor" 2>/dev/null || true
}

if [ "$ACTION" = uninstall ]; then
    say "Quitando Lente de $PREFIX…"
    ID=app.lente.Lente
    as_root rm -f "$PREFIX/bin/lente" \
        "$PREFIX/share/applications/$ID.desktop" \
        "$PREFIX/share/icons/hicolor/scalable/apps/$ID.svg" \
        "$PREFIX/share/metainfo/$ID.metainfo.xml"
    as_root rm -rf "$PREFIX/share/doc/lente" "$PREFIX/share/licenses/lente"
    [ "$UDEV" != "-" ] && as_root rm -f "$UDEV/70-lente-elgato.rules"
    refresh_system
    say "Lente quitado. Tus ajustes siguen en ~/.config/lente-camara (bórralos si quieres)."
    exit 0
fi

HERE=""
if [ "$(basename "$0")" = install.sh ] && [ -f "$0" ]; then
    HERE=$(cd "$(dirname "$0")" && pwd)
fi

if [ -n "$HERE" ] && [ -x "$HERE/lente" ] && [ -f "$HERE/packaging/stage.sh" ]; then
    # Desde el tarball descomprimido.
    BIN="$HERE/lente"
elif [ -n "$HERE" ] && [ -f "$HERE/Cargo.toml" ] && [ -f "$HERE/packaging/stage.sh" ]; then
    # Desde el código fuente.
    command -v cargo >/dev/null 2>&1 || { echo "Falta Rust: instálalo desde https://rustup.rs" >&2; exit 1; }
    say "Compilando Lente…"
    (cd "$HERE" && cargo build --release --locked)
    BIN="$HERE/target/release/lente"
else
    # Por internet: descarga el tarball de la versión y usa su propio instalador.
    case "$REPO" in __*) echo "Este instalador no sabe de qué repositorio descargar: usa LENTE_REPO=usuario/repo" >&2; exit 1 ;; esac
    case "$(uname -m)" in
        x86_64|amd64) ;;
        *) echo "Sólo hay paquete para x86_64. En $(uname -m), compila desde el código: https://github.com/$REPO" >&2; exit 1 ;;
    esac
    if [ -n "$VERSION" ]; then
        URL="https://github.com/$REPO/releases/download/v${VERSION#v}/$ASSET"
    else
        URL="https://github.com/$REPO/releases/latest/download/$ASSET"
    fi
    TMP=$(mktemp -d)
    trap 'rm -rf "$TMP"' EXIT INT TERM
    say "Descargando $URL"
    if command -v curl >/dev/null 2>&1; then
        curl -fL --progress-bar -o "$TMP/$ASSET" "$URL"
    else
        wget -q --show-progress -O "$TMP/$ASSET" "$URL"
    fi
    tar -xzf "$TMP/$ASSET" -C "$TMP"
    INNER=$(find "$TMP" -mindepth 2 -maxdepth 2 -name install.sh | head -n 1)
    [ -n "$INNER" ] || { echo "El paquete descargado no trae install.sh" >&2; exit 1; }
    ARGS=""
    [ "$MODE" = user ] && ARGS="--user"
    sh "$INNER" $ARGS
    exit $?
fi

say "Instalando Lente en $PREFIX…"
as_root sh "$HERE/packaging/stage.sh" / "$PREFIX" "$UDEV" "$BIN"
refresh_system

say "Listo. Abre «Lente» desde el menú o escribe: lente"
if [ "$MODE" = user ]; then
    case ":$PATH:" in *":$HOME/.local/bin:"*) ;; *) echo "Añade ~/.local/bin a tu PATH para usar el comando lente." ;; esac
    echo "Para los ajustes Elgato por USB instala la regla una vez (pide contraseña):"
    echo "  sudo install -Dm644 \"$HERE/70-lente-elgato.rules\" /etc/udev/rules.d/70-lente-elgato.rules && sudo udevadm control --reload-rules && sudo udevadm trigger"
else
    echo "Desconecta y vuelve a conectar la cámara para que tome el permiso USB."
fi
