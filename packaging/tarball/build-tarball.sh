#!/bin/sh
# Builds dist/lente-linux-x86_64.tar.gz: the ready-to-run program with its installer.
# Extract and run ./lente, or install with: sh install.sh  (or sh install.sh --user)
# Usage (from the project folder): sh packaging/tarball/build-tarball.sh
set -eu
cd "$(dirname "$0")/../.."
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
[ -n "${LENTE_SKIP_BUILD:-}" ] || cargo build --release --locked
mkdir -p dist
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT
D="$WORK/lente-$VERSION"
mkdir -p "$D/packaging" "$D/docs"
install -m755 target/release/lente "$D/lente"
install -m755 install.sh "$D/install.sh"
install -m644 packaging/stage.sh packaging/app.lente.Lente.desktop packaging/app.lente.Lente.svg \
    packaging/app.lente.Lente.metainfo.xml "$D/packaging/"
install -m644 70-lente-elgato.rules README.md CHANGELOG.md LICENSE "$D/"
install -m644 docs/COMPATIBILITY.md "$D/docs/"
tar --owner=0 --group=0 --numeric-owner --sort=name -C "$WORK" -czf dist/lente-linux-x86_64.tar.gz "lente-$VERSION"
echo "Done: dist/lente-linux-x86_64.tar.gz (Lente $VERSION)"
