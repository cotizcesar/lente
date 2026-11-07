# Lente RPM package from the already built program (assembled by build-rpm.sh).
%global debug_package %{nil}
%global _build_id_links none

Name:           lente
Version:        %{lente_version}
Release:        1
Summary:        Webcam control (Elgato Facecam and UVC) for Linux
License:        GPL-3.0-or-later
URL:            https://github.com/cotizcesar/lente
BuildArch:      x86_64
# Libraries the window loads at start-up (Fedora and openSUSE names)
Requires:       (libxkbcommon or libxkbcommon0)
Requires:       (mesa-libEGL or Mesa-libEGL1 or libglvnd-egl)
Recommends:     (libwayland-client or libwayland-client0)

%description
Live preview with resolution and frame rate, framing, exposure, focus,
white balance, picture, HDR, noise reduction, low-light mode, status light
and saving settings inside the Elgato camera.
Includes the udev rule for USB access to Elgato cameras.

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
