# Lente

**Webcam control for Linux, made for the Elgato Facecam** (Facecam, Facecam Pro, Facecam Neo,
Facecam MK.2, Facecam 4K) **and any UVC camera.**

Elgato's official app only exists for Windows and macOS. Lente brings the same camera settings to
Linux: live preview with resolution and frame rate, framing, exposure, focus, white balance,
picture, HDR, noise reduction, low-light mode, status light, saving settings inside the camera and
applying the profile the camera stored from Windows.

> Independent project, not affiliated with Elgato or Corsair. "Elgato" and "Facecam" are
> trademarks of Corsair. Lente contains no Elgato code, images or files.

## Install

### From the terminal (any distribution)

```sh
curl -fsSL https://raw.githubusercontent.com/cotizcesar/lente/main/install.sh | sh
```

Installs the latest release into `/usr/local` with its menu entry and the USB permission (asks for
your password). Variants: `… | sh -s -- --user` (just your user, in `~/.local`, no password),
`… | sh -s -- --version 0.4.0`, `… | sh -s -- --uninstall`.

### Packages

On the [releases](https://github.com/cotizcesar/lente/releases) page (all x86_64):

| System | File | How |
|---|---|---|
| Debian, Ubuntu, Mint, Pop!_OS | `lente_0.4.0_amd64.deb` | `sudo apt install ./lente_0.4.0_amd64.deb` |
| Fedora, RHEL, Alma, Rocky | `lente-0.4.0-1.x86_64.rpm` | `sudo dnf install ./lente-0.4.0-1.x86_64.rpm` |
| openSUSE | `lente-0.4.0-1.x86_64.rpm` | `sudo zypper install ./lente-0.4.0-1.x86_64.rpm` |
| Any (Flatpak) | `Lente-0.4.0.flatpak` | `flatpak install --user Lente-0.4.0.flatpak`, then `flatpak run app.lente.Lente` |
| Any (AppImage) | `Lente-0.4.0-x86_64.AppImage` | `chmod +x Lente-*.AppImage` and double-click |
| Any (tarball) | `lente-linux-x86_64.tar.gz` | extract and run `./lente`, or `sh install.sh` to install it |

`SHA256SUMS` holds the checksums (`sha256sum -c SHA256SUMS --ignore-missing`).
The `.deb` and `.rpm` packages, the tarball and the installer include the USB rule; with the
AppImage or Flatpak, install it once as explained in [USB permission](#usb-permission-once).
They run on distributions from 2022 onwards (glibc 2.35). On Arch/CachyOS use the terminal
installer or the AppImage.

### From source

```sh
git clone https://github.com/cotizcesar/lente lente && cd lente && sh install.sh
```

Builds with Rust and installs the same way. More in [docs/BUILDING.md](docs/BUILDING.md).

## Languages

Danish, German, English, Spanish, French, Italian, Japanese, Korean, Dutch, Portuguese (Brazil and
Portugal), Russian, Swedish and Traditional Chinese — the same as the official app. Lente follows
the system language; change it in **Device → Language** or with `LENTE_LANG=de lente`.
Translations live in `i18n/strings.tsv` (one row per text, one column per language).

## Features

| Group | Settings |
|---|---|
| Capture | preview, resolution, 30/60 fps, MJPEG/YUY2/NV12, "fixed frame rate", photo, full screen, grid |
| Framing | zoom, pan, tilt, 4 saved frames with smooth transitions, flip/mirror |
| Exposure | auto/manual, compensation, shutter, gain/ISO, metering, lock, anti-flicker, low-light mode, "adjust once" |
| Focus | auto, distance, metering, lock, "refocus now", 3 focus presets |
| White balance | auto, temperature, tint, lock, "adjust once" |
| Picture | brightness, contrast, saturation, vibrancy, dynamic range (Standard / Wide / HDR) |
| Detail | sharpness, noise reduction, 2D, 3D |
| Device | status light, save to camera, Windows profile, factory reset, restart |

The **Device → What works on Linux** tab shows, for your camera, every feature and how Lente
handles it. The full list is in [docs/COMPATIBILITY.md](docs/COMPATIBILITY.md).

## USB permission (once)

Linux only passes through the Elgato settings the camera advertises. For the rest (tint, vibrancy,
locks, refocus, restart…) Lente talks to the camera directly over USB, as Elgato's app does. That
needs a udev rule. Lente installs it from **Device → Grant permission**, or by hand:

```sh
sudo cp 70-lente-elgato.rules /etc/udev/rules.d/
sudo udevadm control --reload && sudo udevadm trigger
```

Unplug the camera and plug it back in. The rule also turns off USB power saving, which makes the
Facecam disconnect or reject commands.

Your user must be in the `video` group to use `/dev/video*`.

## If it doesn't reach 30 or 60 fps

Lente shows the frame rate the camera reports and the preview's. In low light the camera lengthens
the exposure and drops frames: turn on **Fixed frame rate** under "Video format", or add light.
HDR only works up to 30 fps; Lente switches to 30 when you turn it on.

## Building

See [docs/BUILDING.md](docs/BUILDING.md). Quick start:

```sh
cargo build --release
./target/release/lente
```

## How it works

* Standard settings: V4L2 (`VIDIOC_*_CTRL`).
* Elgato settings: UVC extension units 6 and 7 (`UVCIOC_CTRL_QUERY`) and, when Linux doesn't pass
  them through, direct UVC requests over `usbfs`.
* The numbers (selectors, ranges, rules such as "HDR up to 30 fps" or low-light mode 0/60/62) come
  from the interoperability analysis described in [MECHANICS.md](MECHANICS.md).

Thanks to the open projects [facecam-tui](https://github.com/hamen/facecam-tui) (direct USB
technique) and [ElgatoCam](https://github.com/PrivateGER/ElgatoCam) (Facecam 4K notes).

## License

GPL-3.0-or-later. See [LICENSE](LICENSE).
