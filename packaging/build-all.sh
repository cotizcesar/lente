#!/bin/sh
# Builds every package this machine can make and puts them all in dist/ with SHA256SUMS.
# Packages whose tools are missing are skipped with a note.
#   tarball  .deb  .rpm (rpmbuild)  AppImage (internet)  Flatpak (flatpak-builder)
# Usage (from the project folder): sh packaging/build-all.sh
set -u
cd "$(dirname "$0")/.."
cargo build --release --locked || exit 1
export LENTE_SKIP_BUILD=1
ok=""
skip=""
run() {
    name="$1"; shift
    if "$@"; then ok="$ok $name"; else skip="$skip $name"; fi
}
run tarball sh packaging/tarball/build-tarball.sh
run deb sh packaging/deb/build-deb.sh
if command -v rpmbuild >/dev/null 2>&1; then run rpm sh packaging/rpm/build-rpm.sh; else skip="$skip rpm(no-rpmbuild)"; fi
if [ -z "${LENTE_NO_NETWORK:-}" ]; then run appimage sh packaging/appimage/build-appimage.sh; else skip="$skip appimage(no-network)"; fi
if command -v flatpak-builder >/dev/null 2>&1; then run flatpak sh packaging/flatpak/build-flatpak.sh; else skip="$skip flatpak(no-flatpak-builder)"; fi
cp install.sh dist/install.sh
(cd dist && sha256sum $(ls | grep -v '^SHA256SUMS$') > SHA256SUMS)
echo
echo "Built:$ok"
[ -n "$skip" ] && echo "Skipped:$skip"
ls -la dist
