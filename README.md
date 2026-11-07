# Lente

**Control de cámaras web para Linux, pensado para la Elgato Facecam** (Facecam, Facecam Pro,
Facecam Neo, Facecam MK.2, Facecam 4K) **y cualquier cámara UVC.**

El programa oficial de Elgato sólo existe para Windows y macOS. Lente da en Linux los mismos
ajustes de cámara: vista previa con resolución y fotogramas por segundo, encuadre, exposición,
enfoque, balance de blancos, imagen, HDR, reducción de ruido, modo poca luz, luz de estado,
guardar ajustes dentro de la cámara y aplicar el perfil que la cámara guardó desde Windows.

> Proyecto independiente. No está afiliado a Elgato ni a Corsair. «Elgato» y «Facecam» son
> marcas de Corsair. Lente no contiene código, imágenes ni archivos de Elgato.

## Instalar

### Desde la terminal (cualquier distribución)

```sh
curl -fsSL https://raw.githubusercontent.com/cotizcesar/lente/main/install.sh | sh
```

Instala la última versión en `/usr/local` con su entrada de menú y el permiso USB (pide la
contraseña). Variantes: `… | sh -s -- --user` (sólo tu usuario, en `~/.local`, sin contraseña),
`… | sh -s -- --version 0.4.0`, `… | sh -s -- --uninstall`.

### Paquetes

En la página de [versiones](https://github.com/cotizcesar/lente/releases) (todos para x86_64):

| Sistema | Archivo | Cómo |
|---|---|---|
| Debian, Ubuntu, Mint, Pop!_OS | `lente_0.4.0_amd64.deb` | `sudo apt install ./lente_0.4.0_amd64.deb` |
| Fedora, RHEL, Alma, Rocky | `lente-0.4.0-1.x86_64.rpm` | `sudo dnf install ./lente-0.4.0-1.x86_64.rpm` |
| openSUSE | `lente-0.4.0-1.x86_64.rpm` | `sudo zypper install ./lente-0.4.0-1.x86_64.rpm` |
| Cualquiera (Flatpak) | `Lente-0.4.0.flatpak` | `flatpak install --user Lente-0.4.0.flatpak` y `flatpak run app.lente.Lente` |
| Cualquiera (AppImage) | `Lente-0.4.0-x86_64.AppImage` | `chmod +x Lente-*.AppImage` y doble clic |
| Cualquiera (tarball) | `lente-linux-x86_64.tar.gz` | descomprimir y `./lente`, o `sh install.sh` para instalarlo |

`SHA256SUMS` lleva las sumas de comprobación (`sha256sum -c SHA256SUMS --ignore-missing`).
Los paquetes `.deb`, `.rpm`, el tarball y el instalador traen la regla USB; con AppImage y
Flatpak instálala una vez como se explica en [Permiso USB](#permiso-usb-una-vez).
Funcionan en distribuciones de 2022 en adelante (glibc 2.35). En Arch/CachyOS usa el instalador
por terminal o el AppImage.

### Desde el código

```sh
git clone https://github.com/cotizcesar/lente lente && cd lente && sh install.sh
```

Compila con Rust y lo instala igual. Más en [docs/CONSTRUIR.md](docs/CONSTRUIR.md).

## Idiomas

Danés, alemán, inglés, español, francés, italiano, japonés, coreano, neerlandés, portugués
(Brasil y Portugal), ruso, sueco y chino tradicional — los mismos que la app oficial. Lente usa el
idioma del sistema; se cambia en **Dispositivo → Idioma** o con `LENTE_LANG=de lente`.
Las traducciones están en `i18n/strings.tsv` (una fila por texto, una columna por idioma).

## Funciones

| Grupo | Ajustes |
|---|---|
| Captura | vista previa, resolución, 30/60 fps, MJPEG/YUY2/NV12, «fotogramas fijos», foto, pantalla completa, cuadrícula |
| Encuadre | zoom, pan, tilt, 4 encuadres guardados con transición suave, voltear/espejo |
| Exposición | automática/manual, compensación, obturación, ganancia/ISO, medición, bloqueo, antiparpadeo, modo poca luz, «ajustar una vez» |
| Enfoque | automático, distancia, medición, bloqueo, «reenfocar ahora», 3 preajustes de enfoque |
| Balance de blancos | automático, temperatura, tono, bloqueo, «ajustar una vez» |
| Imagen | brillo, contraste, saturación, intensidad, rango dinámico (Estándar / Amplio / HDR) |
| Detalle | nitidez, reducción de ruido, 2D, 3D |
| Dispositivo | luz de estado, guardar en la cámara, perfil de Windows, restablecer, reiniciar |

La pestaña **Dispositivo → Qué funciona en Linux** muestra, para tu cámara, cada función y cómo
la hace Lente. La lista completa está en [docs/COMPATIBILIDAD.md](docs/COMPATIBILIDAD.md).

## Permiso USB (una vez)

Linux sólo deja usar los ajustes Elgato que la cámara anuncia. Para el resto (tono, intensidad,
bloqueos, reenfocar, reiniciar…) Lente habla con la cámara directamente por USB, igual que el
programa de Elgato. Eso necesita una regla de udev. Lente la instala desde
**Dispositivo → Dar permiso**, o a mano:

```sh
sudo cp 70-lente-elgato.rules /etc/udev/rules.d/
sudo udevadm control --reload && sudo udevadm trigger
```

Desconecta y vuelve a conectar la cámara. La regla también desactiva el ahorro de energía USB,
que hace que la Facecam se desconecte o rechace órdenes.

Tu usuario debe estar en el grupo `video` para usar `/dev/video*`.

## Si no llega a 30 o 60 fps

Lente muestra los fps reales de la cámara y los de la vista previa. Con poca luz la cámara
alarga la exposición y baja los fps: activa **Fotogramas fijos** en «Formato de vídeo» o ilumina
más. El HDR sólo funciona hasta 30 fps; Lente baja a 30 al activarlo.

## Compilar

Ver [docs/CONSTRUIR.md](docs/CONSTRUIR.md). Rápido:

```sh
cargo build --release
./target/release/lente
```

## Cómo funciona

* Ajustes estándar: V4L2 (`VIDIOC_*_CTRL`).
* Ajustes Elgato: unidades de extensión UVC 6 y 7 (`UVCIOC_CTRL_QUERY`) y, si Linux no los
  deja pasar, petición UVC directa por `usbfs`.
* Los números (selectores, rangos, reglas como «HDR hasta 30 fps» o el modo poca luz 0/60/62)
  salen del análisis de interoperabilidad descrito en [MECHANICS.md](MECHANICS.md).

Gracias a los proyectos abiertos
[facecam-tui](https://github.com/hamen/facecam-tui) (técnica USB directa) y
[ElgatoCam](https://github.com/PrivateGER/ElgatoCam) (notas de la Facecam 4K).

## Licencia

GPL-3.0-or-later. Ver [LICENSE](LICENSE).
