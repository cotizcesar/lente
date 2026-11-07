# Paquete RPM de Lente a partir del programa ya compilado (lo arma build-rpm.sh).
%global debug_package %{nil}
%global _build_id_links none

Name:           lente
Version:        %{lente_version}
Release:        1
Summary:        Control de cámaras web (Elgato Facecam y UVC) para Linux
License:        GPL-3.0-or-later
URL:            https://github.com/cotizcesar/lente
BuildArch:      x86_64
# Bibliotecas que la ventana carga al arrancar (nombres de Fedora y de openSUSE)
Requires:       (libxkbcommon or libxkbcommon0)
Requires:       (mesa-libEGL or Mesa-libEGL1 or libglvnd-egl)
Recommends:     (libwayland-client or libwayland-client0)

%description
Vista previa con resolución y fotogramas por segundo, encuadre, exposición,
enfoque, balance de blancos, imagen, HDR, reducción de ruido, modo poca luz,
luz de estado y guardar los ajustes dentro de la cámara Elgato.
Incluye la regla de udev para el acceso USB a las cámaras Elgato.

%install
cp -a %{lente_stage}/. %{buildroot}/

%post
udevadm control --reload-rules >/dev/null 2>&1 || :
udevadm trigger --subsystem-match=usb --attr-match=idVendor=0fd9 >/dev/null 2>&1 || :

%postun
udevadm control --reload-rules >/dev/null 2>&1 || :

%files
%license /usr/share/licenses/lente/LICENSE
/usr/bin/lente
/usr/share/applications/app.lente.Lente.desktop
/usr/share/icons/hicolor/scalable/apps/app.lente.Lente.svg
/usr/share/metainfo/app.lente.Lente.metainfo.xml
/usr/share/doc/lente/
/usr/lib/udev/rules.d/70-lente-elgato.rules
