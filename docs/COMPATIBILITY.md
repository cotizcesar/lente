# What works on Linux

Status of every camera feature of Elgato's Windows app, with the Elgato Facecam 4K
(USB `0fd9:0092`). The app shows this same table for the connected camera, under
**Device → What works on Linux**.

Legend: **Linux** = through the `uvcvideo` driver; **Direct USB** = Lente sends the UVC request to
the camera over `usbfs` (needs the USB permission once); **In Lente** = the app does it;
**Windows only** = not offered.

| Feature | Facecam 4K on Linux | Notes |
|---|---|---|
| Preview | Linux | MJPEG up to 4K60, YUY2 up to 1440p30, NV12 up to 4K30 |
| Resolution and fps | Linux | 30 and 60 fps |
| Photo | In Lente | PNG in Pictures/Lente |
| Zoom, pan, tilt | Linux | |
| Saved frames (4) | In Lente | as in the original: zoom + pan + tilt, same transition timing |
| Flip / mirror | Direct USB | |
| Auto exposure, shutter, gain | Linux | |
| Exposure compensation | Linux | moves brightness while auto exposure is on, as the original does |
| Exposure metering | Linux | average / center / spot |
| Exposure lock | Direct USB | |
| Anti-flicker | Linux | |
| Low-light mode | Linux | Off / below 720p / below 1080p |
| Auto focus, distance | Linux | |
| Focus metering, focus lock, refocus | Direct USB | |
| Focus presets (3) | In Lente | as in the original |
| Auto white balance, temperature | Linux | |
| Tint, white balance lock | Direct USB | |
| Brightness, contrast, saturation, sharpness | Linux | |
| Vibrancy | Direct USB | |
| Dynamic range / HDR | Linux | HDR up to 30 fps |
| Noise reduction | Direct USB | 2D and 3D through Linux |
| Status light | Linux | written exactly as the original does; not yet tested on hardware |
| Save settings to the camera | Linux | read back and checked after saving |
| Profile saved from Windows | Linux | read and applied |
| Factory reset | Linux | |
| Restart the camera | Direct USB | |
| Firmware update | Windows only | firmware isn't distributed and a failed update can brick the camera |
| Recording | Windows only | on Linux, use OBS Studio |
| Teleprompter, chat | Windows only | outside camera control |

This comes from the selectors the Facecam 4K advertises in its USB descriptors (unit 6: 1–3, 6–11,
17, 21, 24; unit 7: 1–9, 11–13, 17, 20–29). Other Facecam models may differ; the app checks when
it opens the camera.
