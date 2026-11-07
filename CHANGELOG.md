# Changelog

## Unreleased

* Documentation in English (README, changelog, build guide, compatibility table, mechanics notes,
  package descriptions and script messages).

## 0.4.0 — 2026-11-06

* **Really saves inside the camera**: "Save as default" always sends the save command to Elgato
  cameras, with the length the camera declares, through the driver or directly over USB (with the
  long wait the camera's memory needs). It then reads back the camera's stored profile and says
  how many settings were stored and which ones weren't.
* **Spaced writes, like the original**: focus, zoom, pan, tilt, temperature and tint are sent to
  the camera at most every 50 ms while dragging (the original's Throttler table), without flooding
  it; the last value always lands, and anything pending is sent before saving to the camera.
* **Frame transitions like the original**: 30 ms steps, speed 1, and a duration based on how far
  the sensor crop moves (3840×2160); zoom moves so the crop's edges travel in straight lines.
* **3 focus presets**, as in the original (previously 4, which was an estimate).
* **Status light written like the original**: Lente sends the same 10 bytes as Elgato's app (for
  streaming and idle) instead of reading and changing one byte; brightness can go down to 0 %.
* **Packages for every distribution**: tarball, `.deb` (Debian/Ubuntu), `.rpm` (Fedora/openSUSE),
  AppImage and Flatpak, built by GitHub for each release, with `SHA256SUMS`. System packages
  include the udev rule for USB access.
* **Terminal install**: `curl -fsSL …/install.sh | sh` (or `--user` without a password,
  `--uninstall` to remove it). Also works from the tarball or from source.

## 0.3.0 — 2026-11-06

* **14 languages**, the same as the official app: Danish, German, English, Spanish, French,
  Italian, Japanese, Korean, Dutch, Portuguese (Brazil and Portugal), Russian, Swedish and
  Traditional Chinese. Picked from the system or in Device → Language. System CJK font for ja/ko/zh.
* **Starts in milliseconds** (~45 ms to the first window frame): the camera opens in the
  background, fonts load on another thread without scanning folders, and failing USB queries no
  longer wait for retries. `abrir-lente.sh` no longer rebuilds when nothing changed.
* Full screen (F11 / Esc / double-click), rule-of-thirds grid, focus presets, smooth transitions
  between frames, shutter speed shown as 1/x s.
* No overflowing text: long labels are cut with "…" and menus fit.

## 0.2.1 — 2026-11-06

* "Factory reset" does what the original does: loads the factory profile (XU2 6 = 0) and saves it
  (XU2 7 = 1).
* Lente shows the resolution and fps the camera itself reports (XU1 status 1), besides the
  measured ones.

## 0.2.0 — 2026-11-06

* Selectable resolution, **30/60 fps** and compression; "Fixed frame rate" so the camera doesn't
  drop fps in low light; the camera's real fps and the preview's on screen.
* Two-thread capture (decoding no longer slows the camera) and MJPEG decoded straight to RGBA;
  BT.709 colors in HD, as on Windows.
* **Windows profile**: reads the settings the camera stored from Elgato's app, compares them with
  the current ones and applies them (also when the camera opens).
* **Direct USB** access for the Elgato settings Linux doesn't pass through: tint, vibrancy, flip,
  noise reduction, focus metering, exposure/focus/white-balance locks, refocus, adjust once,
  restart.
* Save settings inside the camera, factory reset, status light (experimental).
* "What works on Linux" table with the status of each feature.
* **AppImage** and **Flatpak** packages, documentation and automatic publishing on GitHub.

## 0.1.0 — 2026-11-06

* First version: camera list, preview, UVC settings, Elgato settings through extension units,
  dynamic range, low-light mode, exposure compensation, 4 saved frames.
