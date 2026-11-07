# Cambios

## 0.4.0 — 2026-11-06

* **Guardar dentro de la cámara de verdad**: «Guardar como predeterminado» siempre envía la orden
  de guardar a las cámaras Elgato, con la longitud que la cámara declara, por el controlador o
  directo por USB (con la espera larga que necesita la memoria de la cámara). Después lee el perfil
  guardado de la cámara y dice cuántos ajustes quedaron guardados y cuáles no.
* **Escrituras espaciadas como el original**: enfoque, zoom, pan, tilt, temperatura y tono se
  envían a la cámara como mucho cada 50 ms al arrastrar (tabla del Throttler del original), sin
  saturarla; el último valor siempre llega y lo pendiente se envía antes de guardar en la cámara.
* **Transiciones entre encuadres como el original**: pasos de 30 ms, velocidad 1 y duración
  según cuánto se mueve el recorte del sensor (3840×2160); el zoom avanza de forma que los bordes
  del recorte van en línea recta.
* **3 preajustes de enfoque**, como el original (antes 4, que era una estimación).
* **Luz de estado escrita como el original**: Lente envía los mismos 10 bytes que el programa de
  Elgato (al transmitir y en reposo) en vez de leer y cambiar un byte; el brillo admite 0 %.
* **Paquetes para todas las distribuciones**: tarball, `.deb` (Debian/Ubuntu), `.rpm`
  (Fedora/openSUSE), AppImage y Flatpak, construidos por GitHub en cada versión, con `SHA256SUMS`.
  Los paquetes del sistema traen la regla de udev para el acceso USB.
* **Instalación por terminal**: `curl -fsSL …/install.sh | sh` (o `--user` sin contraseña,
  `--uninstall` para quitarlo). También funciona desde el tarball o desde el código fuente.

## 0.3.0 — 2026-11-06

* **14 idiomas**, los mismos de la app oficial: danés, alemán, inglés, español, francés, italiano,
  japonés, coreano, neerlandés, portugués (Brasil y Portugal), ruso, sueco y chino tradicional.
  Se elige solo según el sistema o en Dispositivo → Idioma. Fuente CJK del sistema para ja/ko/zh.
* **Arranque en milisegundos** (~45 ms hasta la primera imagen de la ventana): la cámara se abre
  en segundo plano, las fuentes se cargan en otro hilo sin recorrer carpetas, y las consultas USB
  que fallan ya no esperan reintentos. `abrir-lente.sh` ya no recompila si nada cambió.
* Pantalla completa (F11 / Esc / doble clic), cuadrícula de tercios, 4 preajustes de enfoque,
  transición suave entre encuadres, velocidad de obturación como 1/x s.
* Interfaz sin textos desbordados: los textos largos se recortan con «…» y los menús se ajustan.

## 0.2.1 — 2026-11-06

* «Restablecer de fábrica» hace lo mismo que el original: carga el perfil de fábrica (XU2 6 = 0) y lo guarda (XU2 7 = 1).
* Lente muestra la resolución y los fps que informa la propia cámara (estado XU1 1), además de los medidos.

## 0.2.0 — 2026-11-06

* Resolución, **30/60 fps** y compresión elegibles; «Fotogramas fijos» para que la cámara no baje
  los fps con poca luz; fps reales de la cámara y de la vista previa a la vista.
* Captura en dos hilos (la decodificación ya no frena a la cámara) y MJPEG decodificado
  directamente a RGBA; colores BT.709 en HD como en Windows.
* **Perfil de Windows**: lee los ajustes que la cámara guardó desde la app de Elgato, los compara
  con los actuales y los aplica (también al abrir la cámara).
* Acceso **USB directo** para los ajustes Elgato que Linux no deja pasar: tono, intensidad,
  voltear, reducción de ruido, medición de enfoque, bloqueos de exposición/enfoque/balance,
  reenfocar, ajustar una vez, reiniciar.
* Guardar ajustes dentro de la cámara, restablecer de fábrica, luz de estado (experimental).
* Tabla «Qué funciona en Linux» con el estado de cada función.
* Paquetes **AppImage** y **Flatpak**, documentación y publicación automática en GitHub.

## 0.1.0 — 2026-11-06

* Primera versión: lista de cámaras, vista previa, ajustes UVC, ajustes Elgato por unidades de
  extensión, rango dinámico, modo poca luz, compensación de exposición, 4 encuadres guardados.
