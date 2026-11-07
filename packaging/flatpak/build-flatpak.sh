#!/bin/sh
# Builds dist/Lente-<version>.flatpak (a single file installed with
# flatpak install --user Lente-<version>.flatpak). Needs flatpak-builder and internet.
# Usage (from the project folder): sh packaging/flatpak/build-flatpak.sh
set -eu
cd "$(dirname "$0")/../.."
command -v flatpak-builder >/dev/null 2>&1 || { echo "flatpak-builder is missing (package flatpak-builder)" >&2; exit 1; }
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
mkdir -p dist
flatpak remote-add --user --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo
flatpak install --user -y --noninteractive flathub org.freedesktop.Platform//25.08 \
    org.freedesktop.Sdk//25.08 org.freedesktop.Sdk.Extension.rust-stable//25.08
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT
# Rust dependencies are downloaded beforehand: there is no network inside flatpak-builder.
python3 -m venv "$WORK/venv"
"$WORK/venv/bin/pip" -q install aiohttp toml tomlkit
curl -fsSL -o "$WORK/gen.py" https://raw.githubusercontent.com/flatpak/flatpak-builder-tools/master/cargo/flatpak-cargo-generator.py
"$WORK/venv/bin/python" "$WORK/gen.py" Cargo.lock -o packaging/flatpak/cargo-sources.json
flatpak-builder --user --force-clean --repo="$WORK/repo" "$WORK/build" packaging/flatpak/app.lente.Lente.yml
flatpak build-bundle "$WORK/repo" "dist/Lente-$VERSION.flatpak" app.lente.Lente
echo "Done: dist/Lente-$VERSION.flatpak"
