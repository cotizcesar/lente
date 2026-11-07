#!/bin/sh
# Construye todos los paquetes que se puedan en este equipo y deja todo en dist/
# con SHA256SUMS. Los que faltan herramientas se saltan con un aviso.
#   tarball  .deb  .rpm (rpmbuild)  AppImage (internet)  Flatpak (flatpak-builder)
# Uso (desde la carpeta del proyecto): sh packaging/build-all.sh
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
if command -v rpmbuild >/dev/null 2>&1; then run rpm sh packaging/rpm/build-rpm.sh; else skip="$skip rpm(falta-rpmbuild)"; fi
if [ -z "${LENTE_NO_NETWORK:-}" ]; then run appimage sh packaging/appimage/build-appimage.sh; else skip="$skip appimage(sin-red)"; fi
if command -v flatpak-builder >/dev/null 2>&1; then run flatpak sh packaging/flatpak/build-flatpak.sh; else skip="$skip flatpak(falta-flatpak-builder)"; fi
cp install.sh dist/install.sh
(cd dist && sha256sum $(ls | grep -v '^SHA256SUMS$') > SHA256SUMS)
echo
echo "Hechos:$ok"
[ -n "$skip" ] && echo "Saltados:$skip"
ls -la dist
