#!/bin/sh
# Copia Lente ya compilado a un árbol de instalación. Lo usan el tarball, el .deb, el .rpm
# y el instalador por terminal, para que todos instalen exactamente lo mismo.
#
# Uso: sh packaging/stage.sh RAIZ PREFIJO [CARPETA_UDEV] [BINARIO]
#   RAIZ          carpeta donde se arma el árbol ("" o "/" para instalar de verdad)
#   PREFIJO       /usr (paquetes), /usr/local (instalador) o ~/.local (sin root)
#   CARPETA_UDEV  dónde va la regla USB (por defecto PREFIJO/lib/udev/rules.d; "-" = no instalarla)
#   BINARIO       el programa (por defecto target/release/lente)
set -eu
cd "$(dirname "$0")/.."
ROOT="${1:?falta la carpeta raíz}"
PREFIX="${2:?falta el prefijo}"
UDEV="${3:-$PREFIX/lib/udev/rules.d}"
BIN="${4:-target/release/lente}"
[ -x "$BIN" ] || { echo "Falta $BIN: compila primero con 'cargo build --release'" >&2; exit 1; }
D="${ROOT%/}$PREFIX"
ID=app.lente.Lente

install -Dm755 "$BIN" "$D/bin/lente"
install -Dm644 packaging/$ID.desktop "$D/share/applications/$ID.desktop"
install -Dm644 packaging/$ID.svg "$D/share/icons/hicolor/scalable/apps/$ID.svg"
install -Dm644 packaging/$ID.metainfo.xml "$D/share/metainfo/$ID.metainfo.xml"
install -Dm644 README.md "$D/share/doc/lente/README.md"
install -Dm644 CHANGELOG.md "$D/share/doc/lente/CHANGELOG.md"
install -Dm644 docs/COMPATIBILIDAD.md "$D/share/doc/lente/COMPATIBILIDAD.md"
install -Dm644 LICENSE "$D/share/licenses/lente/LICENSE"
if [ "$UDEV" != "-" ]; then
    install -Dm644 70-lente-elgato.rules "${ROOT%/}$UDEV/70-lente-elgato.rules"
fi
