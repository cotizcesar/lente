#!/bin/sh
# Construye dist/lente_<versión>_amd64.deb (Debian, Ubuntu, Linux Mint, Pop!_OS…).
# Instalar: sudo apt install ./lente_<versión>_amd64.deb
# Usa dpkg-deb si está; si no, arma el mismo formato con tar y xz.
# Uso (desde la carpeta del proyecto): sh packaging/deb/build-deb.sh
set -eu
cd "$(dirname "$0")/../.."
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
ARCH=amd64
[ -n "${LENTE_SKIP_BUILD:-}" ] || cargo build --release --locked
mkdir -p dist
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT
ROOT="$WORK/root"
sh packaging/stage.sh "$ROOT" /usr /usr/lib/udev/rules.d
mkdir -p "$ROOT/DEBIAN"
SIZE=$(du -sk --exclude=DEBIAN "$ROOT" | cut -f1)

cat > "$ROOT/DEBIAN/control" <<EOF
Package: lente
Version: $VERSION
Architecture: $ARCH
Maintainer: Lente <lente@users.noreply.github.com>
Installed-Size: $SIZE
Depends: libc6 (>= 2.35), libgcc-s1, libegl1, libxkbcommon0
Recommends: libwayland-client0, libwayland-egl1, libx11-6, libxcursor1, libxi6, libxrandr2
Section: video
Priority: optional
Description: Control de cámaras web (Elgato Facecam y UVC) para Linux
 Vista previa con resolución y fotogramas por segundo, encuadre, exposición,
 enfoque, balance de blancos, imagen, HDR, reducción de ruido, modo poca luz,
 luz de estado y guardar los ajustes dentro de la cámara Elgato.
 Incluye la regla de udev para el acceso USB a las cámaras Elgato.
EOF

for s in postinst postrm; do
    cat > "$ROOT/DEBIAN/$s" <<'EOF'
#!/bin/sh
set -e
if command -v udevadm >/dev/null 2>&1; then
    udevadm control --reload-rules || true
    udevadm trigger --subsystem-match=usb --attr-match=idVendor=0fd9 || true
fi
exit 0
EOF
    chmod 755 "$ROOT/DEBIAN/$s"
done
(cd "$ROOT" && find . -type f ! -path './DEBIAN/*' | sed 's|^\./||' | sort | xargs md5sum > DEBIAN/md5sums)

OUT="dist/lente_${VERSION}_${ARCH}.deb"
rm -f "$OUT"
if command -v dpkg-deb >/dev/null 2>&1; then
    dpkg-deb --root-owner-group --build "$ROOT" "$OUT"
else
    P="$WORK/pkg"
    mkdir -p "$P"
    printf '2.0\n' > "$P/debian-binary"
    tar --owner=0 --group=0 --numeric-owner --sort=name -C "$ROOT/DEBIAN" -czf "$P/control.tar.gz" .
    tar --owner=0 --group=0 --numeric-owner --sort=name -C "$ROOT" --exclude=./DEBIAN -cJf "$P/data.tar.xz" .
    # Archivo ar clásico, como lo escribe dpkg-deb (nombres sin la barra final de GNU ar).
    {
        printf '!<arch>\n'
        for m in debian-binary control.tar.gz data.tar.xz; do
            size=$(wc -c < "$P/$m" | tr -d ' ')
            printf '%-16s%-12s%-6s%-6s%-8s%-10s`\n' "$m" 0 0 0 100644 "$size"
            cat "$P/$m"
            if [ $((size % 2)) -eq 1 ]; then printf "\n"; fi
        done
    } > "$OUT"
fi
echo "Listo: $OUT"
