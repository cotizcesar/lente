# Compilar y empaquetar Lente

## Requisitos

* Rust estable (`rustup`), versión 1.85 o más nueva.
* Bibliotecas de ventana: en Debian/Ubuntu
  `sudo apt install libxkbcommon-dev libwayland-dev libgl1-mesa-dev`;
  en Fedora `sudo dnf install libxkbcommon-devel wayland-devel mesa-libGL-devel`;
  en Arch/CachyOS ya vienen con el escritorio.

## Programa

```sh
cargo build --release
./target/release/lente            # o: sh abrir-lente.sh
cargo test                        # pruebas
./target/release/lente --screenshot captura.png   # dibuja 120 fotogramas, guarda la ventana y sale
```

## Paquetes

Todos dejan su archivo en `dist/`. Para hacer todos los que se puedan en tu equipo de una vez
(con `SHA256SUMS`):

```sh
sh packaging/build-all.sh
```

| Paquete | Script | Necesita |
|---|---|---|
| Tarball `lente-linux-x86_64.tar.gz` | `sh packaging/tarball/build-tarball.sh` | tar |
| Debian/Ubuntu `lente_<v>_amd64.deb` | `sh packaging/deb/build-deb.sh` | `dpkg-deb`, o `tar` + `xz` |
| Fedora/openSUSE `lente-<v>-1.x86_64.rpm` | `sh packaging/rpm/build-rpm.sh` | `rpmbuild` (`rpm-build` / `rpm`) |
| `Lente-<v>-x86_64.AppImage` | `sh packaging/appimage/build-appimage.sh` | internet (descarga linuxdeploy) |
| `Lente-<v>.flatpak` | `sh packaging/flatpak/build-flatpak.sh` | `flatpak-builder` e internet |

Todos instalan lo mismo, armado por `packaging/stage.sh`: el programa, el icono, la entrada de
menú, los datos para las tiendas de aplicaciones (metainfo), la documentación y la regla de udev
(`/usr/lib/udev/rules.d` en los paquetes, `/etc/udev/rules.d` con el instalador).

Un programa compilado en una distribución reciente pide esa versión de glibc o más nueva. Los
paquetes de cada versión los compila GitHub dentro de Ubuntu 22.04 (glibc 2.35) para que sirvan
en casi todas las distribuciones; los que hagas en tu equipo sirven en el tuyo.

El Flatpak pide `--device=all` para ver las cámaras y su nodo USB. La regla de udev (permiso USB)
no se puede instalar desde dentro del Flatpak: copia el comando desde la app o instala
`70-lente-elgato.rules` a mano (ver README).

## Instalador por terminal

`install.sh` funciona de tres maneras:

* `curl -fsSL https://raw.githubusercontent.com/cotizcesar/lente/main/install.sh | sh`: descarga el
  tarball de la última versión (o `--version X.Y.Z`) y lo instala;
* `sh install.sh` dentro del tarball descomprimido: instala ese programa;
* `sh install.sh` dentro del código fuente: compila con cargo y lo instala.

Opciones: `--user` (en `~/.local`, sin contraseña ni regla USB), `--uninstall`.

## Versiones en GitHub

`.github/workflows/release.yml` construye los seis paquetes al subir una etiqueta y los adjunta a
la versión con `install.sh` y `SHA256SUMS`:

```sh
# 1. sube la versión en Cargo.toml y packaging/app.lente.Lente.metainfo.xml, y anota CHANGELOG.md
git commit -am "Versión 0.4.1"
git tag v0.4.1
git push origin main v0.4.1
```

Para la primera publicación usa `sh publicar-en-github.sh`: sube el código y la etiqueta a
https://github.com/cotizcesar/lente por SSH. Si el repositorio aún no existe, lo crea con `gh`
(si tienes sesión iniciada) o te pide crearlo vacío en https://github.com/new.
