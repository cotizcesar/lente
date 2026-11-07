# Building and packaging Lente

## Requirements

* Stable Rust (`rustup`), 1.85 or newer.
* Window libraries: on Debian/Ubuntu
  `sudo apt install libxkbcommon-dev libwayland-dev libgl1-mesa-dev`;
  on Fedora `sudo dnf install libxkbcommon-devel wayland-devel mesa-libGL-devel`;
  on Arch/CachyOS they come with the desktop.

## The program

```sh
cargo build --release
./target/release/lente            # or: sh abrir-lente.sh
cargo test                        # tests
./target/release/lente --screenshot shot.png   # draws 120 frames, saves the window and exits
```

## Packages

Each script leaves its file in `dist/`. To build every package your machine can make at once
(with `SHA256SUMS`):

```sh
sh packaging/build-all.sh
```

| Package | Script | Needs |
|---|---|---|
| Tarball `lente-linux-x86_64.tar.gz` | `sh packaging/tarball/build-tarball.sh` | tar |
| Debian/Ubuntu `lente_<v>_amd64.deb` | `sh packaging/deb/build-deb.sh` | `dpkg-deb`, or `tar` + `xz` |
| Fedora/openSUSE `lente-<v>-1.x86_64.rpm` | `sh packaging/rpm/build-rpm.sh` | `rpmbuild` (`rpm-build` / `rpm`) |
| `Lente-<v>-x86_64.AppImage` | `sh packaging/appimage/build-appimage.sh` | internet (downloads linuxdeploy) |
| `Lente-<v>.flatpak` | `sh packaging/flatpak/build-flatpak.sh` | `flatpak-builder` and internet |

They all install the same files, laid out by `packaging/stage.sh`: the program, the icon, the menu
entry, the app-store metadata (metainfo), the documentation and the udev rule
(`/usr/lib/udev/rules.d` in packages, `/etc/udev/rules.d` with the installer).

A program built on a recent distribution needs that glibc version or newer. Each release's
packages are built by GitHub inside Ubuntu 22.04 (glibc 2.35) so they work on almost every
distribution; packages you build yourself work on your own machine.

The Flatpak asks for `--device=all` to see the cameras and their USB node. The udev rule (USB
permission) can't be installed from inside the Flatpak: copy the command from the app or install
`70-lente-elgato.rules` by hand (see the README).

## Terminal installer

`install.sh` works three ways:

* `curl -fsSL https://raw.githubusercontent.com/cotizcesar/lente/main/install.sh | sh`: downloads
  the latest release's tarball (or `--version X.Y.Z`) and installs it;
* `sh install.sh` inside the extracted tarball: installs that program;
* `sh install.sh` inside the source tree: builds with cargo and installs it.

Options: `--user` (into `~/.local`, no password and no USB rule), `--uninstall`.

## Releases on GitHub

`.github/workflows/release.yml` builds the packages when a tag is pushed and attaches them to the
release together with `install.sh` and `SHA256SUMS`:

```sh
# 1. bump the version in Cargo.toml and packaging/app.lente.Lente.metainfo.xml, and note it in CHANGELOG.md
git commit -am "Version 0.4.1"
git tag v0.4.1
git push origin main v0.4.1
```

For the first publication there is `sh publish.sh`: it pushes the code and the tag to
https://github.com/cotizcesar/lente over SSH. If the repository doesn't exist yet, it creates it
with `gh` (signing in if needed) or asks you to create it empty at https://github.com/new.
