# Qué funciona en Linux

Estado de cada función de cámara del programa de Elgato para Windows, con la Elgato Facecam 4K
(USB `0fd9:0092`). La app muestra esta misma tabla para la cámara conectada, en
**Dispositivo → Qué funciona en Linux**.

Leyenda: **Linux** = por el controlador `uvcvideo`; **USB directo** = Lente envía la orden UVC a
la cámara por `usbfs` (necesita el permiso USB una vez); **En Lente** = lo hace la app;
**Sólo Windows** = no se ofrece.

| Función | Facecam 4K en Linux | Notas |
|---|---|---|
| Vista previa | Linux | MJPEG hasta 4K60, YUY2 hasta 1440p30, NV12 hasta 4K30 |
| Resolución y fps | Linux | 30 y 60 fps |
| Foto | En Lente | PNG en Imágenes/Lente |
| Zoom, pan, tilt | Linux | |
| Encuadres guardados (4) | En Lente | igual que el original: zoom + pan + tilt |
| Voltear / espejo | USB directo | |
| Exposición automática, obturación, ganancia | Linux | |
| Compensación de exposición | Linux | mueve el brillo con exposición automática, como el original |
| Medición de exposición | Linux | |
| Bloquear exposición | USB directo | |
| Antiparpadeo | Linux | |
| Modo poca luz | Linux | Apagado / por debajo de 720p / por debajo de 1080p |
| Enfoque automático, distancia | Linux | |
| Medición de enfoque, bloquear enfoque, reenfocar | USB directo | |
| Balance automático, temperatura | Linux | |
| Tono, bloquear balance | USB directo | |
| Brillo, contraste, saturación, nitidez | Linux | |
| Intensidad de color | USB directo | |
| Rango dinámico / HDR | Linux | HDR hasta 30 fps |
| Reducción de ruido | USB directo | 2D y 3D por Linux |
| Luz de estado | Linux | experimental: la escritura se dedujo de la lectura |
| Guardar ajustes en la cámara | Linux | |
| Perfil guardado desde Windows | Linux | se lee y se aplica |
| Restablecer de fábrica | Linux | |
| Reiniciar la cámara | USB directo | |
| Actualizar firmware | Sólo Windows | el firmware no se distribuye y un fallo puede inutilizar la cámara |
| Grabación | Sólo Windows | en Linux usa OBS Studio |
| Teleprompter, chat | Sólo Windows | fuera del control de cámara |

Lo anterior sale de los selectores que la Facecam 4K anuncia en sus descriptores USB (unidad 6:
1–3, 6–11, 17, 21, 24; unidad 7: 1–9, 11–13, 17, 20–29). Otras Facecam pueden diferir; la app
lo comprueba al abrir la cámara.
