#!/bin/sh
# Construye dist/Lente-<versión>.flatpak (un archivo que se instala con
# flatpak install --user Lente-<versión>.flatpak). Necesita flatpak-builder e internet.
# Uso (desde la carpeta del proyecto): sh packaging/flatpak/build-flatpak.sh
set -eu
cd "$(dirname "$0")/../.."
command -v flatpak-builder >/dev/null 2>&1 || { echo "Falta flatpak-builder (paquete flatpak-builder)" >&2; exit 1; }
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
mkdir -p dist
flatpak remote-add --user --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo
flatpak install --user -y --noninteractive flathub org.freedesktop.Platform//24.08 \
    org.freedesktop.Sdk//24.08 org.freedesktop.Sdk.Extension.rust-stable//24.08
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT
# Las dependencias de Rust se descargan antes: dentro de flatpak-builder no hay red.
python3 -m venv "$WORK/venv"
"$WORK/venv/bin/pip" -q install aiohttp toml tomlkit
curl -fsSL -o "$WORK/gen.py" https://raw.githubusercontent.com/flatpak/flatpak-builder-tools/master/cargo/flatpak-cargo-generator.py
"$WORK/venv/bin/python" "$WORK/gen.py" Cargo.lock -o packaging/flatpak/cargo-sources.json
flatpak-builder --user --force-clean --repo="$WORK/repo" "$WORK/build" packaging/flatpak/app.lente.Lente.yml
flatpak build-bundle "$WORK/repo" "dist/Lente-$VERSION.flatpak" app.lente.Lente
echo "Listo: dist/Lente-$VERSION.flatpak"
