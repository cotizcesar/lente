#!/bin/sh
# Builds dist/Lente-<version>-x86_64.AppImage with linuxdeploy (downloads it; needs internet).
# Use: chmod +x Lente-*.AppImage and double-click.
# Usage (from the project folder): sh packaging/appimage/build-appimage.sh
set -eu
cd "$(dirname "$0")/../.."
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
[ -n "${LENTE_SKIP_BUILD:-}" ] || cargo build --release --locked
mkdir -p dist
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT
TOOL="$WORK/linuxdeploy-x86_64.AppImage"
curl -fsSL -o "$TOOL" https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/linuxdeploy-x86_64.AppImage
chmod +x "$TOOL"
mkdir -p "$WORK/AppDir/usr/share/metainfo"
cp packaging/app.lente.Lente.metainfo.xml "$WORK/AppDir/usr/share/metainfo/"
export LINUXDEPLOY_OUTPUT_VERSION="$VERSION"
OUT="Lente-$VERSION-x86_64.AppImage"
export OUTPUT="$WORK/$OUT"
"$TOOL" --appimage-extract-and-run \
    --appdir "$WORK/AppDir" \
    --executable target/release/lente \
    --desktop-file packaging/app.lente.Lente.desktop \
    --icon-file packaging/app.lente.Lente.svg \
    --output appimage
mv "$WORK/$OUT" "dist/$OUT"
echo "Done: dist/$OUT"
