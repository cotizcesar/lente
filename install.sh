#!/bin/sh
# Installs Lente from the terminal.
#
#   curl -fsSL https://raw.githubusercontent.com/cotizcesar/lente/main/install.sh | sh
#   curl -fsSL https://raw.githubusercontent.com/cotizcesar/lente/main/install.sh | sh -s -- --user
#
# Options:
#   --user            just for your user, in ~/.local, no password (USB rule separately)
#   --version X.Y.Z   a specific release (default: the latest)
#   --uninstall       remove Lente (add --user if you installed it that way)
#
# Also works from the extracted tarball (sh install.sh) or from the source tree (builds with
# cargo). Installs the program, its icon and menu entry, and the udev rule that gives your
# user USB access to Elgato cameras.
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
        --version) VERSION="${2:?missing version number}"; shift ;;
        --version=*) VERSION="${1#--version=}" ;;
        -h|--help) sed -n '2,16p' "$0" 2>/dev/null || true; exit 0 ;;
        *) echo "Unknown option: $1" >&2; exit 2 ;;
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

# Runs as administrator when needed.
as_root() {
    if [ "$MODE" = user ] || [ "$(id -u)" = 0 ]; then
        "$@"
    elif command -v sudo >/dev/null 2>&1; then
        sudo "$@"
    elif command -v doas >/dev/null 2>&1; then
        doas "$@"
    else
        echo "sudo or doas is needed to install into $PREFIX (or use --user)." >&2
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
    say "Removing Lente from $PREFIX…"
    ID=app.lente.Lente
    as_root rm -f "$PREFIX/bin/lente" \
        "$PREFIX/share/applications/$ID.desktop" \
        "$PREFIX/share/icons/hicolor/scalable/apps/$ID.svg" \
        "$PREFIX/share/metainfo/$ID.metainfo.xml"
    as_root rm -rf "$PREFIX/share/doc/lente" "$PREFIX/share/licenses/lente"
    [ "$UDEV" != "-" ] && as_root rm -f "$UDEV/70-lente-elgato.rules"
    refresh_system
    say "Lente removed. Your settings remain in ~/.config/lente-camara (delete them if you like)."
    exit 0
fi

HERE=""
if [ "$(basename "$0")" = install.sh ] && [ -f "$0" ]; then
    HERE=$(cd "$(dirname "$0")" && pwd)
fi

if [ -n "$HERE" ] && [ -x "$HERE/lente" ] && [ -f "$HERE/packaging/stage.sh" ]; then
    # From the extracted tarball.
    BIN="$HERE/lente"
elif [ -n "$HERE" ] && [ -f "$HERE/Cargo.toml" ] && [ -f "$HERE/packaging/stage.sh" ]; then
    # From the source tree.
    command -v cargo >/dev/null 2>&1 || { echo "Rust is missing: install it from https://rustup.rs" >&2; exit 1; }
    say "Building Lente…"
    (cd "$HERE" && cargo build --release --locked)
    BIN="$HERE/target/release/lente"
else
    # Over the internet: download the release tarball and use its own installer.
    case "$REPO" in __*) echo "This installer doesn't know which repository to download from: use LENTE_REPO=user/repo" >&2; exit 1 ;; esac
    case "$(uname -m)" in
        x86_64|amd64) ;;
        *) echo "Packages exist only for x86_64. On $(uname -m), build from source: https://github.com/$REPO" >&2; exit 1 ;;
    esac
    if [ -n "$VERSION" ]; then
        URL="https://github.com/$REPO/releases/download/v${VERSION#v}/$ASSET"
    else
        URL="https://github.com/$REPO/releases/latest/download/$ASSET"
    fi
    TMP=$(mktemp -d)
    trap 'rm -rf "$TMP"' EXIT INT TERM
    say "Downloading $URL"
    if command -v curl >/dev/null 2>&1; then
        curl -fL --progress-bar -o "$TMP/$ASSET" "$URL"
    else
        wget -q --show-progress -O "$TMP/$ASSET" "$URL"
    fi
    tar -xzf "$TMP/$ASSET" -C "$TMP"
    INNER=$(find "$TMP" -mindepth 2 -maxdepth 2 -name install.sh | head -n 1)
    [ -n "$INNER" ] || { echo "The downloaded package has no install.sh" >&2; exit 1; }
    ARGS=""
    [ "$MODE" = user ] && ARGS="--user"
    sh "$INNER" $ARGS
    exit $?
fi

say "Installing Lente into $PREFIX…"
as_root sh "$HERE/packaging/stage.sh" / "$PREFIX" "$UDEV" "$BIN"
refresh_system

say "Done. Open \"Lente\" from the menu or type: lente"
if [ "$MODE" = user ]; then
    case ":$PATH:" in *":$HOME/.local/bin:"*) ;; *) echo "Add ~/.local/bin to your PATH to use the lente command." ;; esac
    echo "For the Elgato settings over USB, install the rule once (asks for your password):"
    echo "  sudo install -Dm644 \"$HERE/70-lente-elgato.rules\" /etc/udev/rules.d/70-lente-elgato.rules && sudo udevadm control --reload-rules && sudo udevadm trigger"
else
    echo "Unplug the camera and plug it back in so it picks up the USB permission."
fi
