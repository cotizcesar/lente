# Lente — mechanics sheet

Everything below comes from the original program (Elgato Studio 2.0.0.3364), read in
`EGAVDeviceSupport.dll` (the native code that talks to the camera) and
`ElgatoDeviceCapabilities.json`. Lente reimplements it on Linux with V4L2 + UVC. Addresses are
function or data addresses in `EGAVDeviceSupport.dll` unless stated otherwise. Values marked
**estimated** were not found in the code.

## 1. How the original talks to Elgato cameras

* Standard UVC controls (brightness, contrast, saturation, sharpness, white-balance temperature,
  exposure, gain, zoom, pan, tilt, focus, anti-flicker, backlight compensation) through DirectShow
  `IAMVideoProcAmp` / `IAMCameraControl` (strings `IAMPropertyHelper<IAMVideoProcAmp>`,
  `IAMPropertyHelper<IAMCameraControl>`). On Linux: `VIDIOC_QUERYCTRL / G_CTRL / S_CTRL`, with
  ranges read from the camera.
* Elgato's own controls through **UVC extension units (XU)**. The original encodes the unit as
  `0x020N0000` → unit `N`:
  * XU1 = `0x02060000` → unit **6** (`TestAllExtensionUnits` 0x180269530 walks `0x2060000` and
    `0x2070000`, selectors 0..31).
  * XU2 = `0x02070000` → unit **7**.
  * Confirmed on the person's Facecam 4K (USB descriptors): XU unit 6
    `23e49ed0-1178-4f31-ae52-d2fb8a8d3001`, unit 7 `2c49d16a-32b8-4485-3ea8-643a15236002`.
  * Example: `SetHDRMode` 0x1802689c0 writes 1 byte to (0x2060000, selector 0x15).
  On Linux: `UVCIOC_CTRL_QUERY` (SET_CUR 0x01, GET_CUR 0x81, GET_MIN 0x82, GET_MAX 0x83,
  GET_DEF 0x87, GET_INFO 0x86).

## 2. XU control table (56-byte records, table at 0x180662550 and copies)

Record layout: id, name, unit, selector, size, signed, min, max, step, default, flags
(flags 3 = read/write, 1 = read-only).

| Control | XU | Selector | Bytes | Min | Max | Step | Default | Address |
|---|---|---|---|---|---|---|---|---|
| Tint | 1 | 0x12 | 2 (signed) | -300 | 300 | 1 | 0 | 0x180662550 |
| Tint (original Facecam variant) | 1 | 0x12 | 2 | -128 | 128 | 1 | 0 | 0x180660fd0 |
| Vibrancy | 1 | 0x0e | 2 | 0 | 100 | 1 | 0 | 0x180662588 |
| AE Metering | 1 | 0x02 | 1 | 0 | 2 | 1 | 0 | 0x1806625c0 |
| Exposure Time (XU) | 1 | 0x0a | 4 | 1 | 2500 | 1 | 156 | 0x1806625f8 |
| Flip and Mirror | 1 | 0x14 | 1 | 0 | 3 | 1 | 0 | 0x180662668 |
| HDR | 1 | 0x15 | 1 | 0 | 1 | 1 | 0 | 0x1806626a0 |
| Noise Reduction | 1 | 0x16 | 1 | 0 | 3 | 3 | 2 | 0x1806626d8 |
| AF Metering | 1 | 0x19 | 1 | 0 | 2 | 1 | 0 | 0x180662710 |
| 2D Noise Reduction | 2 | 0x03 | 1 | 0 | 100 | 1 | 0 | 0x180662748 |
| 3D Noise Reduction | 2 | 0x14 | 1 | 0 | 100 | 1 | 50 | 0x180662780 |
| Power Line Frequency (XU) | 2 | 0x0e | 1 | 0 | 2 | 1 | 2 | 0x1806627b8 |
| Power Line Freq Mode | 2 | 0x0f | 1 | 0 | 3 | 1 | 0 | 0x1806627f0 |
| USB Transfer Mode | 2 | 0x0b | 1 | 0 | 1 | 1 | 0 | 0x180662828 |
| Binning Mode (low-light mode) | 2 | 0x1d | 2 | 0 | 63 | 1 | 0 | 0x180662860 |
| AE Current Value (read) | 1 | 0x06 | 4 | – | – | – | – | 0x180662898 |
| Gain Value (read) | 1 | 0x08 | 4 | – | – | – | – | 0x180662908 |
| Exposure Time Value (read) | 1 | 0x0b | 4 | 1 | 100000 | – | – | 0x180662940 |
| Focus Value (read) | 1 | 0x0d | 4 | – | – | – | – | 0x180662978 |
| WB Temperature Value (read) | 1 | 0x11 | 4 | – | – | – | – | 0x1806629b0 |

Menus:
* Exposure metering: 0 average, 1 center, 2 spot (confirmed in section 10).
* Focus metering: 0 average, 1 center, 2 spot (`CAMERA_FOCUS_METERING_*`; the order is
  **estimated** for the newer models).
* Anti-flicker: Off / 50 Hz / 60 Hz / Auto (`CAMERA_ANTI_FLICKER_*`). On Linux the camera's
  standard "Power Line Frequency" control is used.

## 3. Rules (read with Ghidra in `EGAVDeviceSupport.dll` and in the IL of `Elgato.Studio.dll`)

* **Dynamic range** (`CameraDeviceElgatoWebcam::SetVirtualDynamicRangeValue` 0x1802cf4a0),
  values 0 Standard / 1 Wide / 2 High (`CAMERA_DYNAMIC_RANGE_*`):
  * value > 2 → error (-4).
  * HDR (prop 0x38 = XU1 0x15) is written as `mode == 2`.
  * BLC (prop 0x1d = 29 = PU_BACKLIGHT_COMPENSATION) is written as `mode == 1`
    (thunk 0x1802cefc0 `return param_1 == 1`), only if the model offers "Wide"
    (0x1802cfbc0: internal model type 2 or 5).
  * Exception: mode 2 on model type 2 with firmware older than 1.61.143 also sets BLC = 1.
  * "Wide" on a model without it → "Rejected - device does not support the Wide option".
  * "High" is refused if the stream runs above 30 fps (0x1e), or 60 (0x3c) on model type 3
    (`FUN_1802cf390`) → "current streaming framerate exceeds this device's HDR limit".
  * Reading (`GetVirtualDynamicRangeProperty` 0x1802cf020 + `FUN_1802cefd0`): HDR = 1 → 2;
    otherwise, with "Wide" available, BLC ≠ 0 → 1; otherwise 0. A 3-option menu (max 2), or 2
    options without "Wide".
  * In Lente: "Wide" appears only if the camera has the backlight control; when HDR is turned on
    above 30 fps, Lente switches to 30 fps.
* **Exposure compensation** (`CameraDeviceElgatoWebcam::SetVirtualEVCompensationValue`
  0x1802ccd70): if auto exposure is off → error -9 ("enable it first"); otherwise clamps the value
  to the range of property 0x14 = 20 = **PU_BRIGHTNESS** and writes it (event 0x43 =
  XU_VIRT_EV_COMPENSATION). In Lente: an "Exposure compensation" control that writes Brightness
  and is disabled while exposure is manual.
* **Manual ISO** only with auto exposure off
  ("SetVirtualISOValue rejected - device auto-exposure mode is active").
* **Low-light mode** = Binning Mode (XU2 0x1d, 2 bytes; written as-is by
  `CElgatoWebcamBaseSupport::SetExtendedBinningMode` 0x1802671d0 and `Camera_SetBinningMode`
  0x180022df4). Options and values in `LowLightSettingViewModel` (Elgato.Studio.dll):
  * `.ctor`: options in the order Off, Below 720p, Below 1080p.
  * `IndexToBitfield`: 0 → **0**, 1 → **60** (0x3C), 2 → **62** (0x3E).
  * `BitfieldToIndex`: 0 → 0; 60 → 1; 62 → 2; any other value → 2 if bit 1 (value 2) is set,
    else 1.
  * Property refreshed: 75 = 0x4b = XU_BINNING_MODE.
* **Frame presets** (`CameraFramePresetController`): `CAMERA_FRAME_PRESET_COUNT` = **4**
  (0x180016f2c), names up to **64** bytes (`CAMERA_FRAME_PRESET_NAME_MAX_LEN` 0x18000d797);
  each preset stores zoom, pan and tilt (`SaveFramePreset` 0x1802a73e0, 3 values per entry).

## 3a. Camera commands (selector and size read from each write function)

Taken from the `(…, 0x020N0000, selector)` call and the buffer size in each
`CElgatoWebcamBaseSupport::Set*` function:

| Command | XU | Selector | Bytes | Function |
|---|---|---|---|---|
| AE_LOCK | 1 | 4 | 1 | 0x1802662a0 |
| AE_LOCK_DELAY | 1 | 5 | 2 | 0x180266450 |
| AE_SPEED | 1 | 3 | 2 | 0x1802667b0 |
| GAIN_LIMIT | 1 | 9 | 4 | 0x180267540 |
| EXPOSURE_TIME_ABSOLUTE | 1 | 10 | 4 | 0x1802660f0 |
| EXPOSURE_PRIORITY | 1 | 12 | 4 | 0x180266600 |
| AWB_LOCK | 1 | 15 | 1 | 0x180266e70 |
| AWB_LOCK_DELAY | 1 | 16 | 2 | 0x180267020 |
| DIGITAL_WINDOW | 1 | 23 | 12 | 0x180267380 |
| PANTILT_ABSOLUTE | 1 | 24 | 8 | 0x180268ef0 |
| AF_LOCK | 1 | 26 | 1 | 0x180266960 |
| AF_LOCK_DELAY | 1 | 27 | 2 | 0x180266b10 |
| ONE_TIME_AUTO (refocus, etc.) | 1 | 30 | 4 | 0x180267c00 |
| VIDEOMODES | 2 | 2 | 65 | 0x18025ff50 |
| LED_CONTROL | 2 | 4 | 10 | 0x180268b70 |
| USER_PROFILE_LOAD | 2 | 6 | 1 | 0x180268660 |
| USER_PROFILE_SAVE | 2 | 7 | 1 | 0x180265a50 |
| USER_PROFILE_CLEAR | 2 | 9 | 1 | 0x1802684b0 |
| MUTE | 2 | 10 | 1 | 0x1802676f0 |
| UVC_INDEX | 2 | 13 | 34 | 0x1802690e0 |
| REBOOT | 2 | 16 | 1 (value 0) | 0x180265cb0 |

(The ones already known match the section 2 table — HDR 21, tint 18, low light 29… — which
validates the extraction.)

* **ONE_TIME_AUTO**: a 4-byte word with `CAMERA_ONE_TIME_AUTO_*` flags (SWIG constants):
  FOCUS = 1 (0x180020a6d), EXPOSURE = 2 (0x18001d601), SHUTTER = 4 (0x180011432),
  WHITE_BALANCE = 8 (0x180007027), TINT = 16 (0x18001c963). Lente: "Refocus now" (1),
  "Adjust white balance once" (8), "Adjust exposure once" (2).
* **Save to the camera**: see section 7.
* **Factory reset**: see §3e (USER_PROFILE_LOAD = 0, then save; 0x1802cb6e0).
* **Restart** (`SendRebootCommand` 0x180265cb0): 1 byte with value 0 to XU2 selector 16.
* **Status light, reading** (`ReadLEDBrightnessAndMode` 0x1802cafe0, `thunk_FUN_1802cb220`,
  `FUN_1802cd830`): the slot is chosen by writing 34 bytes `[4,0,0,0,0,0,0,index,…]` to XU2
  selector 13 (UVC_INDEX), then 10 bytes are read from XU2 selector 4; byte 5 is the brightness
  0..255. Index 3 = while streaming, 2 = idle. Mode: idle > 0 → 2 "always"; only streaming > 0
  → 1 "while streaming"; both 0 → 0 "off". Percent = value·100/255. Writing: see section 12.
* **Frame rate**: the Facecam 4K offers 60 and 30 fps at almost every resolution (MJPEG up to
  4K60, YUY2 up to 1440p30, NV12 up to 4K30; USB descriptors). Lente lists them with
  `VIDIOC_ENUM_FRAMEINTERVALS` and requests them with `VIDIOC_S_PARM`. Default 1080p30 MJPEG
  (**estimated**; 30 fps keeps HDR available). Turning HDR on above 30 fps switches to 30 fps
  instead of refusing; going up to 60 fps with HDR returns dynamic range to Standard.

## 3c. Getting around Linux's limits (internet research)

* **facecam-tui** (github.com/hamen/facecam-tui): the `uvcvideo` driver only allows the selectors
  the camera lists. It sends the same UVC class request directly over `usbfs`
  (`/dev/bus/usb/BBB/DDD`, ioctl `USBDEVFS_CONTROL`) with DEVICE recipient (bmRequestType
  0x20/0xA0), wValue = selector<<8, wIndex = unit<<8 | interface; that way usbfs doesn't need to
  take the interface from `uvcvideo` and it works while the camera streams. It needs a udev rule
  `TAG+="uaccess"` and USB power saving turned off (after waking, the camera rejects requests
  for ~0.5 s with EPIPE; Lente retries writes twice, 250 ms apart — **estimated**).
* **ElgatoCam** (github.com/PrivateGER/ElgatoCam, Facecam 4K): confirms XU1 = unit 6,
  XU2 = unit 7, "save profile" = XU2 selector 7 with 1, and that the camera disconnects with USB
  power saving on.
* Lente: every Elgato control goes through `uvcvideo` first and, if it isn't listed, over direct
  USB. The rule is in `70-lente-elgato.rules` and the app installs it with `pkexec`.

## 3f. Version 0.3.0: transitions, languages and start-up

* **Frame transitions** (`CameraFramePresetController::DoTransition` 0x1802a4860 starts a thread
  with the lambda 0x1802a3c00): see section 9.
* **Focus presets**: **3** (see section 11).
* **Languages**: the 14 `<Resource Language>` entries of the package's `AppxManifest.xml`: DA, DE,
  EN, ES, FR, IT, JA, KO, NL, PT-BR, PT-PT, RU, SV, ZH-HANT. The original's texts are in
  `resources.pri` (per-language string tables, PRI sections 17–29); their order doesn't line up
  across languages without the resource map, so Lente's translations are written separately,
  using the original's visible terminology.
* **Start-up**: window in ~45 ms (measured with `LENTE_TIMING=1`, 46–47 ms warm, 129 ms cold)
  by opening the camera on a thread, loading fonts on another thread without scanning folders,
  and not retrying failing USB reads (read timeout 150 ms, retries only on writes).

## 3e. Save, reset and camera state (read in the code, version 0.2.1)

* `Camera_SaveSettingsToDefault` (export 0x180007cfc) → `CameraDeviceElgatoWebcam` vtable
  (subobject +0x11d0, table 0x1806633a0, slot 0x70) → 0x1802cba10 → **0x1802cb8b0**: writes
  1 byte = **1** to XU2 selector **7**.
* `Camera_ResetToFactoryDefaults` (export 0x1800208c9, slot 0x80) → **0x1802cb6e0**: writes
  1 byte = **0** to XU2 selector **6** (USER_PROFILE_LOAD → factory profile), then saves with
  0x1802cb8b0 (selector 7 = 1) and rereads every setting. The original doesn't use
  USER_PROFILE_CLEAR (selector 9) for this.
* `Camera_SaveUserProfile(byte)` (export 0x1800054ac, slot 0x78) passes on the byte it's given;
  the app (Elgato.Studio.dll) only calls SaveSettingsToDefault and ResetToFactoryDefaults.
* **Camera state** (`CElgatoWebcamBaseSupport::GetCameraState` 0x180261db0): GET_CUR of **13**
  bytes from XU1 selector **1** (CAMERA_STATUS). CAMERA_STATE layout from its SWIG getters:
  byte 0 streaming (0x1800d5560), byte 1 has streamed (0x1800d5530), u16 @2 width
  (0x1800d55e0), u16 @4 height (0x1800d55c0), **u8 @6 fps** (0x1800d55a0), u32 @8 codec
  (0x1800d5580). Lente reads it every second and shows "The camera says: W×H at N fps".
* **JSON profile (XU2 selector 8)**: the original does **not** use this selector (no call with
  unit 7 and selector 8 in `EGAVDeviceSupport.dll`). The original stores its own settings in a
  file on the PC (`EGAVDSSettingsManager::SaveSettingsToFile` 0x1801e4f60). Lente uses selector 8
  following the ElgatoCam research (**estimated**).
* **Exposure priority**: the original doesn't expose CT_AE_PRIORITY; it has `XU1
  EXPOSURE_PRIORITY` (selector 12, 4 bytes, "AE Priority Defaults", range 0..14043 step 14043,
  default 4681; 0x180266600) whose meaning couldn't be read. Lente's "Fixed frame rate" uses the
  standard UVC control (**estimated**).

## 3d. Real frame rates and "looking like Windows" (version 0.2.0)

* **Why it didn't reach 30/60 fps**: (1) capture and decoding ran on the same thread, so a slow
  1080p MJPEG decode held up dequeuing; now one thread only dequeues (and measures the camera's
  real fps) and another decodes the newest frame. (2) In low light the camera lengthens the
  exposure if `exposure_dynamic_framerate` (UVC CT_AE_PRIORITY, V4L2 0x009a0903) is 1; "Fixed
  frame rate" sets it to 0. Lente shows "Camera: N fps · Preview: M fps" and explains the cause.
* **Colors**: YUY2/NV12 are converted with limited-range BT.709 for heights ≥ 720 and BT.601
  below (the Windows convention for HD video; **estimated**). MJPEG uses its own JFIF matrix.
* **Profile stored in the camera** (source: the open ElgatoCam project's Facecam 4K notes, not
  the original program): XU2 selector 8 returns a JSON `{"Brightness":{"value":3,…},…}` of up to
  4096 bytes with the profile Elgato's app saved (selector 7). Lente asks for its length (GET_LEN
  0x85), reads it, compares it with the current values and applies it. Keys: AE_Mode (UVC mask
  1/2/4/8 → V4L2 1/0/2/3), ExpoTime, Gain, Brightness, Contrast, Saturation, Hue, Sharpness,
  Gamma, Backlight, WB_Auto, WB_Temp, PowerFreq, DZoom_Ratio, DPan_Ofst, DTilt_Ofst, AE_Meter,
  NR2D, NR3D, Binning_Mode (index 0..2 → 0/60/62). By default it's applied when the camera opens
  if Lente has no saved settings.

## 3b. What the Facecam 4K accepts on Linux (from its USB descriptors)

Linux (`uvcvideo`) only allows the selectors the camera advertises in each extension unit's
bitmap.
* Unit 6 (XU1) `e7 07 91 00` → selectors 1, 2, 3, 6–11, 17, 21, 24.
* Unit 7 (XU2) `ff 1d f9 1f` → selectors 1–9, 11–13, 17, 20–29.

Through the driver: HDR / dynamic range (21), exposure metering (2), 2D (3) and 3D (20) noise
reduction, USB mode (11), low-light mode / binning (29), gain (8), exposure time (11) and
temperature (17) readings. Not passed by the driver (Lente uses direct USB): tint (18),
vibrancy (14), flip/mirror (20 in XU1), noise reduction (22), focus metering (25), XU
anti-flicker (14 in XU2; the standard UVC anti-flicker is used). The app probes them on open.

## 4. Known Elgato cameras (vendor 0x0fd9, from ElgatoDeviceCapabilities.json)

Facecam 0x78 (USB2 0x77), Facecam Pro 0x79 (0x87), Facecam Neo 0x81,
Facecam 4K 0x92 (0x91), Facecam MK.2 0x94 (0x93), Facecam MK.2 Rev.2 0xd7 (0xd6),
Cam Link 4K 0x66 / 0x7b / 0xa1 / 0xa2, Cam Link 4K MK.2 0xf4 / 0xf5, Cam Link 0x61,
Game Capture HD60 S+ 0x6a, HD60 X 0x82 / 0x8a, Neo 0x8c, 4K S 0xaf / 0xdc.

## 5. Look (the original's Windows dark theme)

Taken from the CSS shipped in the package itself (`PrompterEditor/.../index-BPTyDiVA.css`, block
`[data-platform=windows][data-theme=dark]`), with the same tokens as the WinUI app: background
`#202020`, alternate background / bar `#1c1c1c`, elevated `#2d2d2d`, sidebar `#1a1a1a`, text
`#dddddd` / `#bbbbbb` / `#999999` / `#777777`, border `rgba(255,255,255,.1)`, accent `#60cdff`
(hover `#4ab8e8`), success `#6ccb5f`, warning `#f59e0b`, radii 4 px / 8 px, top bar 48 px.
Original font: Segoe UI Variable (on Linux, the system sans).

Layout, from the capture page names in `Elgato.Studio.dll` / `resources.pri`:
`CapturePage_SplitView` (preview + pane), `CapturePage_DeviceSettingsPane` with `DeviceHeader`
(image + name), `DeviceSettingsTabs`, collapsible groups and `DeviceSettingsFooter` ("Save as
default"); `CameraHudBar` with AE/AF/AWB badges and dynamic range; `CapturePage_FooterCommandBar`
with photo, folder, HUD and pane; `CapturePage_VideoInUseBadge` when another app uses the camera.

## 6. Why there is no image inside Craft Anything

Craft Anything runs in a Flatpak sandbox without `/dev/video*` (only `devices=dri`) and without
permission to start processes on the host, so no app opened from there can read the camera.
Lente detects it (`/.flatpak-info` + an error on open) and explains how to open it outside with
`abrir-lente.sh`.

## 7. Saving inside the camera (how the original does it)

Decompiled from EGAVDeviceSupport.dll:

1. `Camera_SaveSettingsToDefault` (0x180007cfc) → 0x1802cba10: checks the camera is connected
   (0x18028e260).
2. 0x1802cb8b0: first, 0x1802c31d0 releases the links of four properties the camera moves by
   itself while an automatic mode is on (table 0x1806624a0: 2 CT_EXPOSURE_TIME, 53
   XU_EXPOSURE_TIME_ABSOLUTE, 27 PU_WHITE_BALANCE_TEMP, 4 CT_FOCUS; function 0x1802ba2d0 with
   "enable = 0"). This is the Windows app's own bookkeeping. Lente applies the same rule when
   checking: values the camera drives itself (in automatic) aren't compared.
3. 0x18028d6c0 → 0x180273d40 `Throttler::Flush`: sends the camera the changes the original was
   holding back (section 8). Lente does the same: it sends anything pending before saving.
4. Writes XU2 selector 7 = 1: the camera copies its current settings into its memory.
5. 0x180291010: also stores the values in the program's settings file.

Lente does the same and also checks it:

- The payload has the length the camera declares (GET_LEN); the kernel driver refuses any other.
- It goes through the driver and, if that fails, over direct USB. USB writes wait up to 5000 ms,
  like the kernel driver, because writing the camera's memory is slow (**estimated**).
- It waits 400 ms (**estimated**) and reads the camera's stored profile back (XU2 selector 8). It
  compares each manual setting with the live value and reports how many match and which don't.
  If they don't match after going through the driver, it resends over direct USB.
- "Save as default" always tries this on Elgato cameras and shows the real result.
- Factory reset uses the same send (load profile 0, then save).

## 8. Spaced writes (Throttler)

While a slider is dragged, the original doesn't send every position to the camera: it spaces the
writes of some properties. Decompiled:

- Table built at start-up (0x180055a40): property → interval. Six properties, all at **50 ms**
  (0x32): 4 CT_FOCUS, 7 CT_ZOOM, 8 CT_PAN, 10 CT_TILT, 27 PU_WHITE_BALANCE_TEMP,
  50 XU_WHITE_BALANCE_TINT. The numbers are the original's `CAMERA_UVC_PROPERTY_ID`; their names
  are in the table at 0x180643120 (0 CT_AUTO_EXPOSURE_MODE … 20 PU_BRIGHTNESS … 29
  PU_BACKLIGHT_COMPENSATION … 50 XU_WHITE_BALANCE_TINT, 53 XU_EXPOSURE_TIME_ABSOLUTE).
- `CameraDevice::SetUVCValue` (0x180293170) looks up the property's interval (0x18028eb90). If it
  has one, the write goes through the Throttler (0x1802742e0); otherwise it's direct (0x18028b020).
- The Throttler thread (0x180274500) sends an action once `last send + interval × 1,000,000 ns`
  has passed; `Throttler::ApplyPendingAction` (0x180273890) sends the newest value.
- `Throttler::Flush` (0x180273d40) sends everything pending; it's called before saving to the
  camera.

Lente: `set` sends immediately if 50 ms have passed since that setting's last write; otherwise it
keeps the newest value (the interface already shows it) and every frame `pump` sends it once the
50 ms are up. "Save to camera" calls `flush` first. The camera gets at most 20 commands per second
per setting and the last value always lands.

## 9. Frame transitions (real values)

- `CameraFramePresetController` constructor (0x1802a2e60): step = **30 ms** (+0x40 = 0x1e) and
  speed = **1** (+0x44). No function changes them (no export touches them).
- Visible sensor crop for a pan/tilt/zoom (0x1802a0260): zoom ratio = zoom / **100**
  (0x180646d80); width = **3840** / ratio (0x18064efc8), height = **2160** / ratio (0x18064efb8);
  x = (3840 − width) / 2 × (1 + pan), y = (2160 − height) / 2 × (1 − tilt).
- Distance between two crops (0x1802a00c0): the largest of the top-left corner, bottom-right
  corner and center distances, in sensor pixels.
- Duration (`TransitionHelper::GetDuration` 0x1802a0000) = distance × 0.5 (0x180646d40) / speed,
  in ms.
- DoTransition (0x1802a3c00): steps = max(1, duration / 30), one every 30 ms. Zoom moves so that
  1/zoom is linear (0x1802a0440: z0·z1·n / ((n − i)·z1 + i·z0)), so the crop's edges travel in
  straight lines; pan and tilt are recomputed for that crop. The last step writes the exact preset.

Example: at 2× zoom, panning from one end to the other moves the crop (1920 px wide) 1920 px →
960 ms → 32 steps. Lente uses the same math (test `transition_math`). For cameras whose zoom
doesn't start at 100, Lente maps its range onto 1×–4× (like the Facecam's 100–400).

## 10. Exposure and focus metering

- Facecam Pro/4K: `CFacecamProSupport::SetExposureMeteringMode` (0x180374b00) writes the value
  as-is (0, 1 or 2) to XU1; the names (0x180370770) are 0 **average**, 1 **center**, 2 **spot** —
  the same order as Lente's menu.
- Focus: `SetExtendedAFMetering` (0x180266cc0) writes 1 byte to XU1 selector 25.
- Control table: AE metering (0x1806625b0) and AF metering (0x1806626f8) run from 0 to 2 with
  default 0, as in Lente.

## 11. Focus presets

- `Camera_SaveFocusPreset` → interface +0xf8 → 0x1802c2990 → **0x1802ac5e0**: refuses an index
  of 3 or more (`param_2 < 3`), so there are **3** presets. It reads property 4 (CT_FOCUS) and
  stores its current value in the slot (one model converts it first, 0x180535900); then it saves
  the list in the settings (0x1802aca20).
- `Camera_SetFocusPreset` → +0xf0 → 0x1802c2a00 → **0x1802ac850**: checks the index, writes
  CT_FOCUS with the stored value (vtable +0x88) and remembers which one is active (returned by
  `Camera_GetFocusPresetIndexFromDevice`). It doesn't touch autofocus.
- Lente: 3 presets; it stores the current focus distance and writes it back when applied. On
  Linux the driver refuses the distance while autofocus is on, so Lente turns autofocus off first
  (a necessary difference, not from the original).

## 12. Status light: how the original writes it

- Virtual properties: brightness **68** (0x1802c8110: 0–100, step 1, default **100**) and mode
  **72** (0x1802c81f0: 0–2, default **1** = on while streaming). Read with 0x1802cafe0.
- `SetLEDBrightness` (0x1802cbf70) clamps brightness to 0–100; `SetLEDMode` (0x1802cc100) refuses
  a mode of 3 or more. Both call **0x1802c6a40** with (brightness, mode).
- 0x1802c6a40: raw = brightness × 255 / 100. It builds 10 bytes `ff 00 03 00 | 00 raw 00 00 |
  00 00` (byte 2 = slot: 3 while streaming) and sends them (0x1802cd760: XU2 selector 4, 10 bytes,
  with no index selection or read first). Then it changes byte 2 to **2** (idle) and sends again.
  Mode 0: raw 0 in both slots; mode 1: raw while streaming and 0 idle; mode 2: raw in both.
- Lente sends exactly those 10 bytes (test `led_payload`) and then reads the light back as before
  to show what the camera applied. Brightness can be 0 %, as in the original.

## 13. Extension-unit transport (EGAVUVC)

- `EGAVUVC::ReadUVC` (0x180331460) and `EGAVUVC::WriteUVC` (0x180331a70) look up the unit's
  KS node and send one `KsProperty` request (flags 0x10000001 = GET | TOPOLOGY, 0x10000002 =
  SET | TOPOLOGY) through 0x180332d80. They make **a single attempt**: no retry and no delay. A
  failure is logged as `_XU_SetGetExtensionUnit() FAILED with hr = …`; HRESULT 0x80070490
  (element not found) becomes error −15. Only units 6 and 7 are accepted
  (`(unit + 0xfefa0000) & 0xfefeffff == 0`, unit 0x1070000 excluded).
- Lente reads the same way: one attempt (driver, then direct USB). Writes over direct USB are
  retried twice, 250 ms apart, only when the camera answers EPIPE: on Linux, a Facecam waking
  from USB power saving rejects requests for about half a second, a case Windows' driver
  handles. That retry is Linux-specific (**estimated**), not from the original.

## 14. Video mode map (codec sets per model)

- `GetVideoModeMap` (0x1802c9620) picks a table by USB product id (0x1802c9940): Facecam 4K
  (0x91, 0x92) → table 0x180662c70; Facecam MK.2 (0x93, 0x94, 0xd6, 0xd7) → table 0x180662ee0;
  other models have none ("Unknown device … no video mode map").
- Each table has 6 entries (index 0–5) pointing to two 40-byte lists of codec flags per
  resolution/frame-rate slot (0x0e or 0x0c = several codecs, 0x08 = one codec); they feed the
  XU2 VIDEOMODES control (selector 2, 65 bytes, `SetVideoModeCodecs` 0x18025ff50), which changes
  which formats the camera advertises over USB.
- This is a camera configuration, not the app's default preview mode. Lente doesn't write it
  (a wrong value would change what the camera offers to every app); it lists the formats the
  camera advertises. The default preview mode (1080p30 MJPEG) stays **estimated**: no default
  was found in the native library.

