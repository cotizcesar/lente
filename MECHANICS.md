# Facecam Studio (Linux) — hoja de mecánicas

Todo lo de abajo viene del programa original (Elgato Studio 2.0.0.3364), leído en
`EGAVDeviceSupport.dll` (código nativo que habla con la cámara) y
`ElgatoDeviceCapabilities.json`. El programa Linux lo reimplementa con V4L2 + UVC.

## 1. Cómo habla el original con las cámaras Elgato

* Controles UVC estándar (brillo, contraste, saturación, nitidez, temperatura de
  blancos, exposición, ganancia, zoom, pan, tilt, enfoque, antiparpadeo, compensación
  de contraluz) por DirectShow `IAMVideoProcAmp` / `IAMCameraControl`
  (strings `IAMPropertyHelper<IAMVideoProcAmp>`, `IAMPropertyHelper<IAMCameraControl>`).
  En Linux: `VIDIOC_QUERYCTRL / G_CTRL / S_CTRL`, rangos leídos de la cámara.
* Controles propios de Elgato por **unidades de extensión UVC (XU)**.
  El original codifica la unidad como `0x020N0000` → unidad `N`:
  * XU1 = `0x02060000` → unidad **6** (función `TestAllExtensionUnits` 0x180269530 recorre
    `0x2060000` y `0x2070000`, selectores 0..31).
  * XU2 = `0x02070000` → unidad **7**.
  * Confirmado en la Facecam 4K de la persona (descriptores USB): XU unidad 6
    `23e49ed0-1178-4f31-ae52-d2fb8a8d3001`, unidad 7 `2c49d16a-32b8-4485-3ea8-643a15236002`.
  * Ejemplo: `SetHDRMode` 0x1802689c0 escribe 1 byte en (0x2060000, selector 0x15).
  En Linux: `UVCIOC_CTRL_QUERY` (SET_CUR 0x01, GET_CUR 0x81, GET_MIN 0x82,
  GET_MAX 0x83, GET_DEF 0x87, GET_INFO 0x86).

## 2. Tabla de controles XU (descriptor de 56 bytes, tabla en 0x180662550 y copias)

Formato de cada registro: id, nombre, unidad, selector, tamaño, con signo, mín, máx, paso, defecto, flags
(flags 3 = lectura/escritura, 1 = sólo lectura).

| Control | XU | Selector | Bytes | Mín | Máx | Paso | Defecto | Dirección |
|---|---|---|---|---|---|---|---|---|
| Tint (tono) | 1 | 0x12 | 2 (con signo) | -300 | 300 | 1 | 0 | 0x180662550 |
| Tint (variante Facecam original) | 1 | 0x12 | 2 | -128 | 128 | 1 | 0 | 0x180660fd0 |
| Vibrancy (intensidad) | 1 | 0x0e | 2 | 0 | 100 | 1 | 0 | 0x180662588 |
| AE Metering (medición exposición) | 1 | 0x02 | 1 | 0 | 2 | 1 | 0 | 0x1806625c0 |
| Exposure Time (XU) | 1 | 0x0a | 4 | 1 | 2500 | 1 | 156 | 0x1806625f8 |
| Flip and Mirror | 1 | 0x14 | 1 | 0 | 3 | 1 | 0 | 0x180662668 |
| HDR | 1 | 0x15 | 1 | 0 | 1 | 1 | 0 | 0x1806626a0 |
| Noise Reduction | 1 | 0x16 | 1 | 0 | 3 | 3 | 2 | 0x1806626d8 |
| AF Metering (medición enfoque) | 1 | 0x19 | 1 | 0 | 2 | 1 | 0 | 0x180662710 |
| 2D Noise Reduction | 2 | 0x03 | 1 | 0 | 100 | 1 | 0 | 0x180662748 |
| 3D Noise Reduction | 2 | 0x14 | 1 | 0 | 100 | 1 | 50 | 0x180662780 |
| Power Line Frequency (XU) | 2 | 0x0e | 1 | 0 | 2 | 1 | 2 | 0x1806627b8 |
| Power Line Freq Mode | 2 | 0x0f | 1 | 0 | 3 | 1 | 0 | 0x1806627f0 |
| USB Transfer Mode | 2 | 0x0b | 1 | 0 | 1 | 1 | 0 | 0x180662828 |
| Binning Mode (modo poca luz) | 2 | 0x1d | 2 | 0 | 63 | 1 | 0 | 0x180662860 |
| AE Current Value (lectura) | 1 | 0x06 | 4 | – | – | – | – | 0x180662898 |
| Gain Value (lectura) | 1 | 0x08 | 4 | – | – | – | – | 0x180662908 |
| Exposure Time Value (lectura) | 1 | 0x0b | 4 | 1 | 100000 | – | – | 0x180662940 |
| Focus Value (lectura) | 1 | 0x0d | 4 | – | – | – | – | 0x180662978 |
| WB Temperature Value (lectura) | 1 | 0x11 | 4 | – | – | – | – | 0x1806629b0 |

Menús (nombres de enumeraciones en `Elgato.Studio.Common.dll`):
* Medición de exposición: 0 Promedio, 1 Centro ponderado, 2 Puntual
  (`CAMERA_EXPOSURE_METERING_AVERAGE / CENTER_WEIGHTED_AVERAGE / SPOT`; orden **estimado**).
  Las etiquetas vienen de una lista de opciones del propio DLL (`center_weighted`,
  `average`, `spot`, `center` en 0x1806524b0…0x1806524dc, referenciadas desde 0x1806615d0…);
  no se pudo leer con seguridad qué valor crudo lleva cada una.
* Medición de enfoque: 0 Promedio, 1 Centro, 2 Puntual (`CAMERA_FOCUS_METERING_*`; orden **estimado**).
* Antiparpadeo: Desactivado / 50 Hz / 60 Hz / Auto (`CAMERA_ANTI_FLICKER_*`).
  En Linux se usa el control estándar "Power Line Frequency" de la cámara.

## 3. Reglas (leídas con Ghidra en `EGAVDeviceSupport.dll` y en el IL de `Elgato.Studio.dll`)

* **Rango dinámico** (`CameraDeviceElgatoWebcam::SetVirtualDynamicRangeValue` 0x1802cf4a0),
  valores 0 Estándar / 1 Amplio / 2 Alto (`CAMERA_DYNAMIC_RANGE_*`):
  * valor > 2 → error (-4).
  * HDR (prop 0x38 = XU1 0x15) se escribe = `mode == 2`.
  * BLC (prop 0x1d = 29 = PU_BACKLIGHT_COMPENSATION) se escribe = `mode == 1`
    (thunk 0x1802cefc0 `return param_1 == 1`), sólo si el modelo ofrece "Amplio"
    (0x1802cfbc0: tipo de modelo interno 2 o 5).
  * Excepción: modo 2 en el tipo de modelo 2 con firmware anterior a 1.61.143 también pone BLC = 1.
  * "Amplio" en un modelo sin soporte → "Rejected - device does not support the Wide option".
  * "Alto" se rechaza si los fps del stream superan 30 (0x1e), o 60 (0x3c) en el tipo de modelo 3
    (`FUN_1802cf390`) → "current streaming framerate exceeds this device's HDR limit".
  * Lectura (`GetVirtualDynamicRangeProperty` 0x1802cf020 + `FUN_1802cefd0`): HDR = 1 → 2;
    si no, con "Amplio" disponible, BLC ≠ 0 → 1; si no → 0. Menú de 3 opciones (máx. 2),
    o 2 opciones si no hay "Amplio".
  * En Linux: "Amplio" aparece sólo si la cámara tiene el control de contraluz; HDR se rechaza
    por encima de 60 fps y avisa por encima de 30 fps (no se sabe qué tipo interno es la Facecam 4K).
* **Compensación de exposición** (`CameraDeviceElgatoWebcam::SetVirtualEVCompensationValue`
  0x1802ccd70): si la exposición automática está inactiva → error -9 ("enable it first");
  si no, recorta el valor al rango de la propiedad 0x14 = 20 = **PU_BRIGHTNESS** y la escribe
  (evento 0x43 = XU_VIRT_EV_COMPENSATION). 20 = Brillo se deduce de la numeración del enum
  `CAMERA_UVC_PROPERTY_ID` (BLC = 0x1d = 29 en 0x1802cf4a0 → Brillo = 29 − 9 = 20).
  En Linux: control "Compensación de exposición" que escribe Brillo y queda desactivado
  con exposición manual.
* **ISO manual** sólo con exposición automática desactivada
  ("SetVirtualISOValue rejected - device auto-exposure mode is active").
* **Brillo del LED** (`ReadLEDBrightnessAndMode` 0x1802cafe0): porcentaje = valor·100/255,
  por defecto 100 %. No se usa en la versión Linux (selector no confirmado).
* **Guardar como predeterminado**: el original usa XU2 USER_PROFILE_SAVE (selector no
  encontrado). En Linux se guarda un perfil en `~/.config/lente-camara/` y se aplica
  al abrir la cámara.
* **Modo poca luz** = Binning Mode (XU2 0x1d, 2 bytes; escrito tal cual por
  `CElgatoWebcamBaseSupport::SetExtendedBinningMode` 0x1802671d0 y
  `Camera_SetBinningMode` 0x180022df4). Opciones y valores en
  `LowLightSettingViewModel` (Elgato.Studio.dll):
  * `.ctor`: opciones en orden Apagado, Por debajo de 720p, Por debajo de 1080p.
  * `IndexToBitfield`: 0 → **0**, 1 → **60** (0x3C), 2 → **62** (0x3E).
  * `BitfieldToIndex`: 0 → 0; 60 → 1; 62 → 2; otro valor → 2 si el bit 1 (valor 2) está, si no 1.
  * Propiedad que refresca: 75 = 0x4b = XU_BINNING_MODE.
* **Encuadres** (`CameraFramePresetController`): `CAMERA_FRAME_PRESET_COUNT` = **4**
  (0x180016f2c), nombre de hasta **64** bytes (`CAMERA_FRAME_PRESET_NAME_MAX_LEN` 0x18000d797);
  cada encuadre guarda zoom, pan y tilt (`SaveFramePreset` 0x1802a73e0, 3 valores por entrada).

## 3a. Órdenes de la cámara (selector y tamaño leídos de cada función de escritura)

Extraídos de la llamada `(…, 0x020N0000, selector)` y del tamaño del búfer en cada función
`CElgatoWebcamBaseSupport::Set*` de `EGAVDeviceSupport.dll`:

| Orden | XU | Selector | Bytes | Función |
|---|---|---|---|---|
| AE_LOCK (bloquear exposición) | 1 | 4 | 1 | 0x1802662a0 |
| AE_LOCK_DELAY | 1 | 5 | 2 | 0x180266450 |
| AE_SPEED | 1 | 3 | 2 | 0x1802667b0 |
| GAIN_LIMIT | 1 | 9 | 4 | 0x180267540 |
| EXPOSURE_TIME_ABSOLUTE | 1 | 10 | 4 | 0x1802660f0 |
| EXPOSURE_PRIORITY | 1 | 12 | 4 | 0x180266600 |
| AWB_LOCK (bloquear balance) | 1 | 15 | 1 | 0x180266e70 |
| AWB_LOCK_DELAY | 1 | 16 | 2 | 0x180267020 |
| DIGITAL_WINDOW | 1 | 23 | 12 | 0x180267380 |
| PANTILT_ABSOLUTE | 1 | 24 | 8 | 0x180268ef0 |
| AF_LOCK (bloquear enfoque) | 1 | 26 | 1 | 0x180266960 |
| AF_LOCK_DELAY | 1 | 27 | 2 | 0x180266b10 |
| ONE_TIME_AUTO (reenfocar, etc.) | 1 | 30 | 4 | 0x180267c00 |
| VIDEOMODES | 2 | 2 | 65 | 0x18025ff50 |
| LED_CONTROL | 2 | 4 | 10 | 0x180268b70 |
| USER_PROFILE_LOAD | 2 | 6 | 1 | 0x180268660 |
| USER_PROFILE_SAVE | 2 | 7 | 1 | 0x180265a50 |
| USER_PROFILE_CLEAR | 2 | 9 | 1 | 0x1802684b0 |
| MUTE | 2 | 10 | 1 | 0x1802676f0 |
| UVC_INDEX | 2 | 13 | 34 | 0x1802690e0 |
| REBOOT | 2 | 16 | 1 (valor 0) | 0x180265cb0 |

(Las ya conocidas coinciden con la tabla de la sección 2: HDR 21, tono 18, poca luz 29…, lo
que valida la extracción.)

* **ONE_TIME_AUTO**: palabra de 4 bytes con banderas `CAMERA_ONE_TIME_AUTO_*`
  (constantes SWIG): FOCUS = 1 (0x180020a6d), EXPOSURE = 2 (0x18001d601), SHUTTER = 4
  (0x180011432), WHITE_BALANCE = 8 (0x180007027), TINT = 16 (0x18001c963).
  Lente: «Reenfocar ahora» (1), «Ajustar balance una vez» (8), «Ajustar exposición una vez» (2).
* **Guardar en la cámara** (`SaveUserProfileTo` 0x180265a50): 1 byte a XU2 sel 7. El valor
  que pasa el llamador no se ve aquí; el proyecto abierto ElgatoCam documenta que Camera Hub
  escribe 1 en la Facecam 4K. «Guardar como predeterminado» hace esto y además guarda en Lente.
* **Restablecer de fábrica**: ver §3e (USER_PROFILE_LOAD = 0 y luego guardar; 0x1802cb6e0).
* **Reiniciar** (`SendRebootCommand` 0x180265cb0): 1 byte con valor 0 a XU2 sel 16.
* **Luz de estado** (`ReadLEDBrightnessAndMode` 0x1802cafe0, `thunk_FUN_1802cb220`,
  `FUN_1802cd830`): se elige el registro escribiendo 34 bytes `[4,0,0,0,0,0,0,índice,…]` en
  XU2 sel 13 (UVC_INDEX) y se leen 10 bytes de XU2 sel 4; el byte 5 es el brillo 0..255.
  Índice 3 = al transmitir, 2 = en reposo. Modo: reposo > 0 → 2 «siempre»; sólo transmitir > 0
  → 1 «al transmitir»; ambos 0 → 0 «apagada». Porcentaje = valor·100/255. La escritura (mismo
  registro, byte 5 cambiado) **no** se vio en el código: es experimental.
* **Fotogramas por segundo**: la Facecam 4K ofrece 60 y 30 fps en casi todas las resoluciones
  (MJPEG hasta 4K60, YUY2 hasta 1440p30, NV12 hasta 4K30; descriptores USB). Lente los lista
  con `VIDIOC_ENUM_FRAMEINTERVALS` y los pide con `VIDIOC_S_PARM`. Por defecto 1080p30 MJPEG
  (**estimado**; 30 fps deja el HDR disponible). Al activar HDR con más de 30 fps, Lente baja a
  30 fps en lugar de rechazarlo; al subir a 60 fps con HDR, el rango dinámico vuelve a Estándar.

## 3c. Cómo se saltan los límites de Linux (investigación en internet)

* Proyecto **facecam-tui** (github.com/hamen/facecam-tui): el controlador `uvcvideo` sólo deja
  usar los selectores listados por la cámara. Envía la misma petición de clase UVC directamente
  por `usbfs` (`/dev/bus/usb/BBB/DDD`, ioctl `USBDEVFS_CONTROL`) con destinatario DISPOSITIVO
  (bmRequestType 0x20/0xA0), wValue = selector<<8, wIndex = unidad<<8 | interfaz; así usbfs no
  necesita quitarle la interfaz a `uvcvideo` y funciona mientras la cámara transmite. Necesita
  una regla udev `TAG+="uaccess"` y desactivar el ahorro de energía USB (tras despertar, la
  cámara rechaza peticiones ~0,5 s con EPIPE; Lente reintenta 3 veces cada 200 ms).
* Proyecto **ElgatoCam** (github.com/PrivateGER/ElgatoCam, Facecam 4K): confirma XU1 = unidad 6,
  XU2 = unidad 7, «guardar perfil» = XU2 sel 7 con 1, y que la cámara se desconecta con el
  ahorro de energía USB activo.
* Lente: cada control Elgato va primero por `uvcvideo` y, si no está listado, por USB directo.
  La regla está en `70-lente-elgato.rules` y la app la instala con `pkexec`.

## 3f. Versión 0.3.0: transiciones, idiomas y arranque

* **Transición entre encuadres** (`CameraFramePresetController::DoTransition` 0x1802a4860 lanza un
  hilo con la lambda 0x1802a3c00): calcula la distancia entre ventanas de inicio y fin en un
  espacio de sensor de **3840 × 2160** (0xF00 × 0x870, `FUN_1802a0260`), la duración con
  `TransitionHelper::GetDuration` (0x1802a0000) = distancia × **0.5** (doble en 0x180646d40) /
  velocidad, el número de pasos = duración / intervalo del controlador (+0x40, ms), y en cada paso
  escribe pan (prop 8), tilt (prop 10) y zoom (prop 7) interpolados (0x1802a03e0 /
  0x1802a0440), duerme el intervalo y al final fija los valores exactos. Velocidad (1),
  intervalo (30 ms) y la geometría del recorte: ver la sección 9.
* **Preajustes de enfoque**: **3** (ver sección 11).
* **Idiomas**: los 14 `<Resource Language>` del `AppxManifest.xml` del paquete: DA, DE, EN, ES,
  FR, IT, JA, KO, NL, PT-BR, PT-PT, RU, SV, ZH-HANT. Los textos del original están en `resources.pri`
  (tablas de cadenas por idioma: secciones 17–29 del PRI); su orden no coincide entre idiomas sin
  el mapa de recursos, así que las traducciones de Lente están escritas aparte, usando la
  terminología visible del original (p. ej. «Modo de poca luz», «Antiparpadeo»,
  «Belichtungskorrektur», «Auf Kamera speichern»).
* **Arranque**: ventana en ~45 ms (medido con `LENTE_TIMING=1`, 46–47 ms en caliente, 129 ms en
  frío) gracias a abrir la cámara en un hilo, cargar fuentes en otro hilo sin recorrer carpetas y
  no reintentar lecturas USB que fallan (timeout de lectura 150 ms, reintento sólo en escrituras).

## 3e. Guardar, restablecer y estado de la cámara (leído en el código, versión 0.2.1)

* `Camera_SaveSettingsToDefault` (export 0x180007cfc) → vtable `CameraDeviceElgatoWebcam`
  (subobjeto +0x11d0, tabla 0x1806633a0, ranura 0x70) → 0x1802cba10 → **0x1802cb8b0**:
  escribe 1 byte = **1** en XU2 selector **7**.
* `Camera_ResetToFactoryDefaults` (export 0x1800208c9, ranura 0x80) → **0x1802cb6e0**: escribe
  1 byte = **0** en XU2 selector **6** (USER_PROFILE_LOAD → perfil de fábrica), luego guarda con
  0x1802cb8b0 (selector 7 = 1) y vuelve a leer todos los ajustes. *Corrige la versión anterior de
  Lente, que usaba USER_PROFILE_CLEAR (selector 9); el original no usa ese selector para esto.*
* `Camera_SaveUserProfile(byte)` (export 0x1800054ac, ranura 0x78) pasa el byte que le dan;
  la app (Elgato.Studio.dll) sólo llama a SaveSettingsToDefault y ResetToFactoryDefaults.
* **Estado de la cámara** (`CElgatoWebcamBaseSupport::GetCameraState` 0x180261db0): GET_CUR
  de **13** bytes en XU1 selector **1** (CAMERA_STATUS). Estructura CAMERA_STATE según sus
  getters SWIG: byte 0 transmitiendo (0x1800d5560), byte 1 ya transmitió (0x1800d5530),
  u16 @2 ancho (0x1800d55e0), u16 @4 alto (0x1800d55c0), **u8 @6 fps** (0x1800d55a0),
  u32 @8 códec (0x1800d5580). Lente lo lee cada segundo y muestra «La cámara dice: W×H a N fps».
  (Que el búfer crudo tenga ya esa forma es una deducción: la función copia los 13 bytes directo
  a la estructura.)
* **Perfil JSON (XU2 selector 8)**: el original **no** usa este selector (no hay ninguna llamada
  con unidad 7 y selector 8 en `EGAVDeviceSupport.dll`). El original guarda sus propios ajustes
  en un archivo del equipo (`EGAVDSSettingsManager::SaveSettingsToFile` 0x1801e4f60), así que
  «verse como en Windows» puede depender también de ajustes que sólo existen en ese PC.
  Lente sigue usando el selector 8 según la investigación de ElgatoCam (**estimado**).
* **Prioridad de exposición**: el original no expone CT_AE_PRIORITY en su interfaz; tiene
  `XU1 EXPOSURE_PRIORITY` (selector 12, 4 bytes, «AE Priority Defaults», rango 0..14043 paso 14043,
  defecto 4681; 0x180266600) cuyo significado no se pudo leer. «Fotogramas fijos» de Lente usa el
  control estándar UVC (**estimado**).

## 3d. Fotogramas reales y «verse como en Windows» (versión 0.2.0)

* **Por qué no llegaba a 30/60 fps**: (1) la captura y la decodificación iban en el mismo hilo,
  así que un MJPEG 1080p lento frenaba el desencolado; ahora un hilo sólo desencola (mide los fps
  reales de la cámara) y otro decodifica el fotograma más nuevo. (2) Con poca luz la cámara alarga
  la exposición si `exposure_dynamic_framerate` (UVC CT_AE_PRIORITY, V4L2 0x009a0903) vale 1;
  «Fotogramas fijos» lo pone a 0. Lente muestra «Cámara: N fps · Vista previa: M fps» y explica
  la causa. (3) `abrir-lente.sh` usa ahora la compilación optimizada.
* **Colores**: YUY2/NV12 se convierten con BT.709 de rango limitado para alturas ≥ 720 y BT.601
  por debajo (convención de Windows para vídeo HD). MJPEG va con su propia matriz JFIF.
* **Perfil guardado en la cámara** (fuente: notas del proyecto abierto ElgatoCam para la
  Facecam 4K, no el programa original): XU2 selector 8 devuelve un JSON
  `{"Brightness":{"value":3,…},…}` de hasta 4096 bytes con el perfil que guardó la app de Elgato
  (selector 7). Lente pide su longitud (GET_LEN 0x85), lo lee, lo compara con los valores actuales
  y lo aplica. Claves: AE_Mode (máscara UVC 1/2/4/8 → V4L2 1/0/2/3), ExpoTime, Gain, Brightness,
  Contrast, Saturation, Hue, Sharpness, Gamma, Backlight, WB_Auto, WB_Temp, PowerFreq,
  DZoom_Ratio, DPan_Ofst, DTilt_Ofst, AE_Meter, NR2D, NR3D, Binning_Mode (índice 0..2 → 0/60/62).
  Por defecto se aplica al abrir la cámara si no hay ajustes guardados en Lente.

## 3b. Qué acepta la Facecam 4K en Linux (observado en sus descriptores USB)

Linux (`uvcvideo`) sólo deja usar los selectores que la cámara anuncia en el mapa de bits
de cada unidad de extensión.
* Unidad 6 (XU1) `e7 07 91 00` → selectores 1, 2, 3, 6–11, 17, 21, 24.
* Unidad 7 (XU2) `ff 1d f9 1f` → selectores 1–9, 11–13, 17, 20–29.

Funcionan: HDR / rango dinámico (21), medición de exposición (2), reducción de ruido 2D (3)
y 3D (20), modo USB (11), modo poca luz / binning (29), lecturas de ganancia (8),
tiempo de exposición (11) y temperatura (17).
No disponibles en Linux: tono (18), intensidad (14), voltear/espejo (20 en XU1),
reducción de ruido general (22), medición de enfoque (25), antiparpadeo XU (14 en XU2;
se usa el antiparpadeo estándar UVC). La app los prueba al abrir y los lista aparte.

## 4. Cámaras Elgato conocidas (vendor 0x0fd9, de ElgatoDeviceCapabilities.json)

Facecam 0x78 (USB2 0x77), Facecam Pro 0x79 (0x87), Facecam Neo 0x81,
Facecam 4K 0x92 (0x91), Facecam MK.2 0x94 (0x93), Facecam MK.2 Rev.2 0xd7 (0xd6),
Cam Link 4K 0x66 / 0x7b / 0xa1 / 0xa2, Cam Link 4K MK.2 0xf4 / 0xf5, Cam Link 0x61,
Game Capture HD60 S+ 0x6a, HD60 X 0x82 / 0x8a, Neo 0x8c, 4K S 0xaf / 0xdc.

## 5. Aspecto (tema oscuro de Windows del original)

Tomado del CSS que trae el propio paquete (`PrompterEditor/.../index-BPTyDiVA.css`,
bloque `[data-platform=windows][data-theme=dark]`), con los mismos tokens que la app WinUI:
fondo `#202020`, fondo alterno / barra `#1c1c1c`, elevado `#2d2d2d`, barra lateral `#1a1a1a`,
texto `#dddddd` / `#bbbbbb` / `#999999` / `#777777`, borde `rgba(255,255,255,.1)`,
acento `#60cdff` (hover `#4ab8e8`), éxito `#6ccb5f`, aviso `#f59e0b`, radios 4 px / 8 px,
barra superior 48 px. Fuente original: Segoe UI Variable (en Linux, la sans del sistema).

Distribución, según los nombres de la página de captura en `Elgato.Studio.dll` / `resources.pri`:
`CapturePage_SplitView` (vista previa + panel), `CapturePage_DeviceSettingsPane` con
`DeviceHeader` (imagen + nombre), `DeviceSettingsTabs`, grupos expandibles y
`DeviceSettingsFooter` («Guardar como predeterminado»); `CameraHudBar` con insignias
AE/AF/AWB y rango dinámico; `CapturePage_FooterCommandBar` con foto, carpeta, HUD y panel;
`CapturePage_VideoInUseBadge` cuando otra app usa la cámara.

## 6. Por qué no hay imagen dentro de Craft Anything

Craft Anything corre en un sandbox Flatpak sin `/dev/video*` (sólo `devices=dri`) y sin
permiso para lanzar procesos en el sistema, así que ninguna app abierta desde ahí puede
leer la cámara. Lente lo detecta (`/.flatpak-info` + error al abrir) y explica cómo abrirla
fuera con `abrir-lente.sh`. La Facecam 4K anuncia MJPEG, YUY2 y NV12 hasta 3840×2160
(descriptores USB); Lente pide MJPEG 1280×720 y cambia a YUY2 si MJPEG falla.

## 7. Guardar dentro de la cámara (cómo lo hace el original)

Decompilado de EGAVDeviceSupport.dll:

1. `Camera_SaveSettingsToDefault` (0x180007cfc) → 0x1802cba10: comprueba que la cámara está conectada (0x18028e260).
2. 0x1802cb8b0: primero 0x1802c31d0 suelta los enlaces de cuatro propiedades que la cámara mueve sola mientras un modo automático está activo (tabla 0x1806624a0: 2 CT_EXPOSURE_TIME, 53 XU_EXPOSURE_TIME_ABSOLUTE, 27 PU_WHITE_BALANCE_TEMP, 4 CT_FOCUS; función 0x1802ba2d0 con «activar = 0»). Es contabilidad interna del programa de Windows. Lente aplica la misma regla al comprobar: los valores que la cámara mueve sola (en automático) no se comparan.
3. 0x18028d6c0 → 0x180273d40 `Throttler::Flush`: envía a la cámara los cambios que el original retenía para no saturarla (sección 8). Lente hace lo mismo: envía lo pendiente antes de guardar.
4. Escribe XU2 selector 7 = 1: la cámara copia sus ajustes actuales a su memoria.
5. 0x180291010: además guarda los valores en el archivo de ajustes del programa.

Lente hace lo mismo y además lo comprueba:

- El dato enviado tiene la longitud que la cámara declara (GET_LEN); el controlador del kernel rechaza cualquier otra.
- Va por el controlador y, si falla, directo por USB. Las escrituras por USB esperan 5000 ms, como el controlador del kernel, porque escribir la memoria de la cámara es lento.
- Espera 400 ms y vuelve a leer el perfil guardado de la cámara (XU2 selector 8). Compara cada ajuste manual con el valor en vivo y dice cuántos coinciden y cuáles no. Si no coinciden tras ir por el controlador, lo reenvía directo por USB.
- «Guardar como predeterminado» siempre lo intenta en cámaras Elgato (antes sólo si el camino ya estaba confirmado) y muestra el resultado real.
- Restablecer de fábrica usa el mismo envío (carga el perfil 0 y guarda).

## 8. Escrituras espaciadas (Throttler)

Al arrastrar un deslizador, el original no manda cada posición a la cámara: espacia las escrituras
de algunas propiedades. Decompilado:

- Tabla creada al arrancar (0x180055a40): propiedad → intervalo. Seis propiedades, todas con
  **50 ms** (0x32): 4 CT_FOCUS, 7 CT_ZOOM, 8 CT_PAN, 10 CT_TILT, 27 PU_WHITE_BALANCE_TEMP,
  50 XU_WHITE_BALANCE_TINT. Los números son el `CAMERA_UVC_PROPERTY_ID` del original; sus nombres
  están en la tabla 0x180643120 (0 CT_AUTO_EXPOSURE_MODE … 20 PU_BRIGHTNESS … 29
  PU_BACKLIGHT_COMPENSATION … 50 XU_WHITE_BALANCE_TINT, 53 XU_EXPOSURE_TIME_ABSOLUTE).
- `CameraDevice::SetUVCValue` (0x180293170) busca el intervalo de la propiedad (0x18028eb90). Si
  tiene, la escritura pasa por el Throttler (0x1802742e0); si no, va directa (0x18028b020).
- El hilo del Throttler (0x180274500) manda una acción cuando `último envío + intervalo × 1 000 000 ns`
  ya pasó; `Throttler::ApplyPendingAction` (0x180273890) envía el valor más reciente.
- `Throttler::Flush` (0x180273d40) manda todo lo pendiente; se llama antes de guardar en la cámara.

Lente: `set` envía al momento si pasaron 50 ms desde la última escritura de ese ajuste; si no,
guarda el valor más nuevo (la interfaz ya lo muestra) y cada fotograma `pump` lo envía al cumplirse
los 50 ms. «Guardar en la cámara» llama antes a `flush`. Así la cámara recibe como mucho 20 órdenes
por segundo por ajuste y el último valor siempre llega.

## 9. Transiciones entre encuadres (valores reales)

Antes eran estimadas (velocidad 0.5, paso 33 ms, límites 150–1500 ms). Ahora salen del código:

- Constructor de `CameraFramePresetController` (0x1802a2e60): paso = **30 ms** (+0x40 = 0x1e) y
  velocidad = **1** (+0x44). No hay función que los cambie (ninguna exportación los toca).
- Recorte visible del sensor para un pan/tilt/zoom (0x1802a0260): razón de zoom = zoom / **100**
  (0x180646d80); ancho = **3840** / razón (0x18064efc8), alto = **2160** / razón (0x18064efb8);
  x = (3840 − ancho) / 2 × (1 + pan), y = (2160 − alto) / 2 × (1 − tilt).
- Distancia entre dos recortes (0x1802a00c0): la mayor de la esquina superior izquierda, la
  inferior derecha y el centro, en píxeles del sensor.
- Duración (TransitionHelper::GetDuration 0x1802a0000) = distancia × 0.5 / velocidad, en ms.
- DoTransition (0x1802a3c00): pasos = máx(1, duración / 30), uno cada 30 ms. El zoom avanza de
  modo que 1/zoom es lineal (0x1802a0440: z0·z1·n / ((n − i)·z1 + i·z0)), así los bordes del
  recorte van en línea recta; pan y tilt se recalculan para ese recorte. El último paso escribe
  el encuadre exacto.

Ejemplo: con zoom 2× y pan de un extremo al otro el recorte (1920 px) se mueve 1920 px →
960 ms → 32 pasos. Lente usa estas mismas cuentas (prueba `transition_math`). Para cámaras cuyo
zoom no empieza en 100, Lente lleva su rango a 1×–4× (como el 100–400 de la Facecam).

## 10. Medición de exposición y de enfoque

- Facecam Pro/4K: `CFacecamProSupport::SetExposureMeteringMode` (0x180374b00) escribe el valor
  tal cual (0, 1 o 2) en XU1; los nombres (0x180370770) son 0 **promedio**, 1 **centro**,
  2 **puntual**: el mismo orden del menú de Lente.
- Enfoque: `SetExtendedAFMetering` (0x180266cc0) escribe 1 byte en XU1 selector 25.
- Tabla de controles: medición AE (0x1806625b0) y AF (0x1806626f8) van de 0 a 2 con valor
  inicial 0, como en Lente.


## 11. Preajustes de enfoque

- `Camera_SaveFocusPreset` → interfaz +0xf8 → 0x1802c2990 → **0x1802ac5e0**: rechaza un índice
  de 3 o más (`param_2 < 3`), así que hay **3** preajustes. Lee la propiedad 4 (CT_FOCUS) y guarda
  su valor actual en la ranura (en un modelo lo convierte antes, 0x180535900); luego guarda la
  lista en los ajustes (0x1802aca20).
- `Camera_SetFocusPreset` → +0xf0 → 0x1802c2a00 → **0x1802ac850**: comprueba el índice, escribe
  CT_FOCUS con el valor guardado (vtable +0x88) y recuerda cuál está activo (lo devuelve
  `Camera_GetFocusPresetIndexFromDevice`). No toca el enfoque automático.
- Lente: 3 preajustes, guarda la distancia de enfoque actual y la escribe al aplicar. En Linux el
  controlador rechaza la distancia mientras el enfoque automático está activo, así que Lente lo
  apaga antes (diferencia necesaria, no del original).

## 12. Luz de estado: cómo la escribe el original

- Propiedades virtuales: brillo **68** (0x1802c8110: 0–100, paso 1, inicial **100**) y modo **72**
  (0x1802c81f0: 0–2, inicial **1** = encendida al transmitir). Se leen con 0x1802cafe0.
- `SetLEDBrightness` (0x1802cbf70) recorta el brillo a 0–100; `SetLEDMode` (0x1802cc100) rechaza
  un modo de 3 o más. Los dos llaman a **0x1802c6a40** con (brillo, modo).
- 0x1802c6a40: raw = brillo × 255 / 100. Arma 10 bytes `ff 00 03 00 | 00 raw 00 00 | 00 00`
  (byte 2 = ranura: 3 al transmitir) y los envía (0x1802cd760: XU2 selector 4, 10 bytes, sin
  elegir índice antes ni leer). Luego cambia el byte 2 a **2** (en reposo) y lo envía otra vez.
  Modo 0: raw 0 en las dos ranuras; modo 1: raw al transmitir y 0 en reposo; modo 2: raw en las dos.
- Lente escribía antes leyendo y cambiando el byte 5 tras elegir la ranura con UVC_INDEX
  (**estimado**). Ahora envía exactamente esos 10 bytes (prueba `led_payload`) y después lee la
  luz como antes para mostrar lo que la cámara aplicó. El brillo admite 0 %, como el original.

