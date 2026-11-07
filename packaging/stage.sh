#!/bin/sh
# Copies the built Lente into an install tree. Used by the tarball, the .deb, the .rpm and the
# terminal installer, so they all install exactly the same files.
#
# Usage: sh packaging/stage.sh ROOT PREFIX [UDEV_DIR] [BINARY]
#   ROOT      folder the tree is laid out in ("" or "/" to really install)
#   PREFIX    /usr (packages), /usr/local (installer) or ~/.local (no root)
#   UDEV_DIR  where the USB rule goes (default PREFIX/lib/udev/rules.d; "-" = don't install it)
#   BINARY    the program (default target/release/lente)
set -eu
cd "$(dirname "$0")/.."
ROOT="${1:?missing root folder}"
PREFIX="${2:?missing prefix}"
UDEV="${3:-$PREFIX/lib/udev/rules.d}"
BIN="${4:-target/release/lente}"
[ -x "$BIN" ] || { echo "$BIN is missing: build it first with 'cargo build --release'" >&2; exit 1; }
D="${ROOT%/}$PREFIX"
ID=app.lente.Lente

install -Dm755 "$BIN" "$D/bin/lente"
install -Dm644 packaging/$ID.desktop "$D/share/applications/$ID.desktop"
install -Dm644 packaging/$ID.svg "$D/share/icons/hicolor/scalable/apps/$ID.svg"
install -Dm644 packaging/$ID.metainfo.xml "$D/share/metainfo/$ID.metainfo.xml"
install -Dm644 README.md "$D/share/doc/lente/README.md"
install -Dm644 CHANGELOG.md "$D/share/doc/lente/CHANGELOG.md"
install -Dm644 docs/COMPATIBILITY.md "$D/share/doc/lente/COMPATIBILITY.md"
install -Dm644 LICENSE "$D/share/licenses/lente/LICENSE"
if [ "$UDEV" != "-" ]; then
    install -Dm644 70-lente-elgato.rules "${ROOT%/}$UDEV/70-lente-elgato.rules"
fi
