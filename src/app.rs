//! The window, laid out like the original's capture page: device rail on the left,
//! a large preview with a HUD bar and a footer command bar, and the device-settings pane
//! on the right with tabs, expanders and a "save as defaults" footer.
//! Colours come from the original package's dark Windows theme (see MECHANICS.md §5).

use crate::camera::{self, Camera, CameraEntry, Kind, Profile};
use crate::elgato::{self, Group};
use crate::stream::Preview;
use crate::v4l;
use egui::{pos2, Align, Align2, Color32, CornerRadius, FontId, Layout, Margin, Pos2, Rect, RichText, Sense, Stroke, StrokeKind, Vec2};
use std::time::Instant;
use crate::i18n::{tr, tr_n, trf};

// Dark Windows theme tokens shipped with the original (PrompterEditor CSS,
// [data-platform=windows][data-theme=dark]).
pub const BG: Color32 = Color32::from_rgb(0x20, 0x20, 0x20); // --color-bg
pub const BG_ALT: Color32 = Color32::from_rgb(0x1c, 0x1c, 0x1c); // --color-bg-alt / navbar
pub const ELEVATED: Color32 = Color32::from_rgb(0x2d, 0x2d, 0x2d); // --color-bg-elevated
pub const SIDEBAR: Color32 = Color32::from_rgb(0x1a, 0x1a, 0x1a); // --color-bg-sidebar
pub const CARD: Color32 = Color32::from_rgb(0x2b, 0x2b, 0x2b);
pub const ACCENT: Color32 = Color32::from_rgb(0x60, 0xcd, 0xff); // --color-accent
pub const ACCENT_HOVER: Color32 = Color32::from_rgb(0x4a, 0xb8, 0xe8);
pub const TEXT: Color32 = Color32::from_rgb(0xdd, 0xdd, 0xdd); // --color-text-primary
pub const TEXT_2: Color32 = Color32::from_rgb(0xbb, 0xbb, 0xbb); // --color-text-secondary
pub const TEXT_DIM: Color32 = Color32::from_rgb(0x99, 0x99, 0x99); // --color-text-muted
pub const TEXT_FAINT: Color32 = Color32::from_rgb(0x77, 0x77, 0x77);
pub const SUCCESS: Color32 = Color32::from_rgb(0x6c, 0xcb, 0x5f);
pub const WARNING: Color32 = Color32::from_rgb(0xf5, 0x9e, 0x0b);
fn border() -> Color32 {
    Color32::from_white_alpha(26) // rgba(255,255,255,.1)
}
fn hover_bg() -> Color32 {
    Color32::from_white_alpha(15) // rgba(255,255,255,.06)
}
fn active_bg() -> Color32 {
    Color32::from_white_alpha(20)
}

pub struct Shot {
    pub path: String,
    pub frames: u32,
    pub requested: bool,
    pub done: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

#[derive(PartialEq, Clone, Copy)]
enum Tab {
    Camera,
    Device,
}

pub struct App {
    entries: Vec<CameraEntry>,
    selected: usize,
    cam: Option<Camera>,
    open_error: Option<String>,
    profile: Profile,
    preview: Option<Preview>,
    preview_on: bool,
    texture: Option<egui::TextureHandle>,
    last_seq: u64,
    fmt_choice: Mode,
    confirm: Option<camera::Command>,
    led_edit: Option<camera::Led>,
    tried_fallback: bool,
    mirror_preview: bool,
    hud_on: bool,
    pane_open: bool,
    tab: Tab,
    status: String,
    status_time: Instant,
    last_readout: Instant,
    pub shot: Option<Shot>,
    open_groups: [bool; 7],
    sandboxed: bool,
    /// Camera being opened on a worker thread (index, result channel), so the window shows at once.
    opening: Option<(usize, std::sync::mpsc::Receiver<Opened>)>,
    started: Instant,
    timing_reported: bool,
    fullscreen: bool,
    grid_on: bool,
    transition: Option<Transition>,
}

/// A frame-preset move in progress (CameraFramePresetController::DoTransition in the original).
struct Transition {
    /// Control indexes of pan, tilt and zoom (any may be missing).
    pan: Option<usize>,
    tilt: Option<usize>,
    zoom: Option<usize>,
    from: [f64; 4],
    to: [f64; 4],
    z0: f64,
    z1: f64,
    steps: u32,
    step: u32,
    /// Exact preset values, written on the last step.
    target: Vec<(usize, i32)>,
    last_step: Instant,
}

/// Control value ↔ the original's units: pan/tilt −1..1, zoom ratio (value / 100 when the
/// camera counts from 100, as UVC zoom does).
fn ptz_norm(c: &camera::Control, v: i32) -> f64 {
    let span = (c.max - c.min).max(1) as f64;
    2.0 * (v - c.min) as f64 / span - 1.0
}
fn ptz_denorm(c: &camera::Control, n: f64) -> i32 {
    let span = (c.max - c.min).max(1) as f64;
    (c.min as f64 + (n + 1.0) / 2.0 * span).round() as i32
}
fn zoom_ratio(c: &camera::Control, v: i32) -> f64 {
    if c.min as f64 >= elgato::ZOOM_UNIT {
        v as f64 / elgato::ZOOM_UNIT
    } else {
        // Cameras whose zoom starts at 0: map the range onto 1×–4× like the Facecam's 100–400.
        1.0 + 3.0 * (v - c.min) as f64 / (c.max - c.min).max(1) as f64
    }
}
fn zoom_value(c: &camera::Control, ratio: f64) -> i32 {
    if c.min as f64 >= elgato::ZOOM_UNIT {
        (ratio * elgato::ZOOM_UNIT).round() as i32
    } else {
        (c.min as f64 + (ratio - 1.0) / 3.0 * (c.max - c.min) as f64).round() as i32
    }
}

/// What the worker thread hands back after opening a camera and applying saved settings.
struct Opened {
    cam: Result<Camera, String>,
    profile: Profile,
    msg: Option<String>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, shot: Option<Shot>) -> Self {
        let started = *crate::START.get_or_init(Instant::now);
        // Fonts are read on a worker thread: the first frame never waits for the disk.
        let fctx = cc.egui_ctx.clone();
        std::thread::spawn(move || {
            if let Some(fonts) = build_fonts() {
                fctx.set_fonts(fonts);
                fctx.request_repaint();
            }
        });
        style(&cc.egui_ctx);
        let mut entries = camera::discover();
        if entries.is_empty() {
            entries.push(camera::demo_entry());
        }
        let selected = entries
            .iter()
            .position(|e| e.error.is_none() && e.elgato_webcam)
            .or_else(|| entries.iter().position(|e| e.error.is_none()))
            .or_else(|| entries.iter().position(|e| e.elgato_webcam))
            .unwrap_or(0);
        let mut app = App {
            entries,
            selected,
            cam: None,
            open_error: None,
            profile: Profile::default(),
            preview: None,
            preview_on: shot.is_none(),
            texture: None,
            last_seq: 0,
            fmt_choice: (0, 0, 0, 0, 0),
            confirm: None,
            led_edit: None,
            tried_fallback: false,
            mirror_preview: false,
            hud_on: true,
            pane_open: true,
            tab: if std::env::var("LENTE_TAB").as_deref() == Ok("dispositivo") { Tab::Device } else { Tab::Camera },
            status: String::new(),
            status_time: Instant::now(),
            last_readout: Instant::now(),
            shot,
            open_groups: [true, true, false, false, true, false, false],
            // Inside another app's Flatpak (e.g. Craft Anything) there are no cameras; our own Flatpak has --device=all.
            sandboxed: std::path::Path::new("/.flatpak-info").exists() && std::env::var("FLATPAK_ID").as_deref() != Ok("app.lente.Lente"),
            opening: None,
            started,
            timing_reported: false,
            fullscreen: false,
            grid_on: false,
            transition: None,
        };
        app.open_selected(&cc.egui_ctx);
        app
    }

    fn say(&mut self, s: impl Into<String>) {
        self.status = s.into();
        self.status_time = Instant::now();
    }

    fn offline(&self) -> bool {
        self.cam.as_ref().map(|c| c.dev.is_none()).unwrap_or(true)
    }

    /// Start opening the selected camera on a worker thread (USB queries take tens of ms each).
    fn open_selected(&mut self, ctx: &egui::Context) {
        self.preview = None;
        self.texture = None;
        self.cam = None;
        self.tried_fallback = false;
        let Some(entry) = self.entries.get(self.selected).cloned() else { return };
        self.open_error = entry.error.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        let c2 = ctx.clone();
        std::thread::spawn(move || {
            let profile = camera::load_profile(&entry.profile_id());
            let mut msg = None;
            let cam = Camera::open(&entry).map(|mut cam| {
                if profile.use_camera_profile && profile.defaults.is_none() && cam.camera_profile.is_some() {
                    let errs = cam.apply_camera_profile();
                    msg = Some(if errs.is_empty() { tr("Ajustes de la cámara (los de Windows) aplicados") } else { tr_n("Ajustes aplicados con {} aviso(s)", errs.len()) });
                }
                if profile.apply_on_open && cam.dev.is_some() {
                    if let Some(vals) = profile.defaults.clone() {
                        let errs = cam.apply_values(&vals);
                        msg = Some(if errs.is_empty() { tr("Ajustes guardados aplicados") } else { tr_n("Ajustes aplicados con {} aviso(s)", errs.len()) });
                    }
                }
                cam
            });
            let _ = tx.send(Opened { cam, profile, msg });
            c2.request_repaint();
        });
        self.opening = Some((self.selected, rx));
    }

    /// Pick up a camera the worker finished opening.
    fn poll_opening(&mut self, ctx: &egui::Context) {
        let Some((idx, rx)) = &self.opening else { return };
        let idx = *idx;
        let Ok(o) = rx.try_recv() else { return };
        self.opening = None;
        if idx != self.selected {
            return;
        }
        self.profile = o.profile;
        if let Some(m) = o.msg {
            self.say(m);
        }
        match o.cam {
            Ok(cam) => {
                self.fmt_choice = self.profile.video_mode.filter(|m| mode_valid(&cam.formats, *m)).unwrap_or_else(|| pick_format(&cam.formats));
                self.led_edit = cam.led;
                self.cam = Some(cam);
                self.restart_preview(ctx);
            }
            Err(e) => self.open_error = Some(format!("{}: {e}", tr("No se pudo abrir la cámara"))),
        }
    }

    fn restart_preview(&mut self, ctx: &egui::Context) {
        self.preview = None;
        self.texture = None;
        let Some(cam) = &self.cam else { return };
        if !self.preview_on || cam.dev.is_none() || self.fmt_choice.0 == 0 {
            return;
        }
        let (f, w, h, n, d) = self.fmt_choice;
        self.preview = Some(Preview::start(cam.entry.path.clone(), f, w, h, (n, d), ctx.clone()));
    }

    /// If MJPEG frames can't be decoded, fall back to uncompressed YUYV once.
    fn check_fallback(&mut self, ctx: &egui::Context) {
        if self.tried_fallback {
            return;
        }
        let failed = self
            .preview
            .as_ref()
            .map(|p| p.shared.lock().unwrap().error.as_deref().map(|e| e.contains("decodificar")).unwrap_or(false))
            .unwrap_or(false);
        if !failed {
            return;
        }
        self.tried_fallback = true;
        let mut next = None;
        if let Some(cam) = &self.cam {
            if let Some(f) = cam.formats.iter().find(|f| f.fourcc == v4l::FMT_YUYV) {
                let s = f.sizes.iter().find(|s| **s == (1280, 720)).or(f.sizes.last()).copied().unwrap_or((640, 480));
                let iv = f.fps_for(s.0, s.1).last().copied().unwrap_or((0, 0));
                next = Some((v4l::FMT_YUYV, s.0, s.1, iv.0, iv.1));
            }
        }
        if let Some(n) = next {
            self.fmt_choice = n;
            self.say(tr("MJPEG no se pudo decodificar; usando YUY2"));
            self.restart_preview(ctx);
        }
    }

    /// Change resolution / frame rate / compression. The original refuses HDR above 30 fps
    /// (FUN_1802cf390), so a faster mode turns the dynamic range back to Standard.
    fn apply_mode(&mut self, m: Mode, ctx: &egui::Context) {
        self.fmt_choice = m;
        self.profile.video_mode = Some(m);
        self.save_profile();
        let fps = v4l::fps_value((m.3, m.4));
        if let Some(cam) = &mut self.cam {
            if let Some(i) = cam.find("dynamic_range") {
                if cam.controls[i].value == 2 && fps > elgato::HDR_MAX_FPS + 0.5 {
                    let _ = cam.set(i, 0);
                    self.say(tr_n("A {} fps no hay HDR: el rango dinámico pasó a Estándar", format!("{fps:.0}")));
                }
            }
        }
        self.restart_preview(ctx);
    }

    /// Lower the frame rate to the HDR limit, keeping resolution and compression.
    fn hdr_mode(&self) -> Option<Mode> {
        let cam = self.cam.as_ref()?;
        let m = self.fmt_choice;
        let f = cam.formats.iter().find(|f| f.fourcc == m.0)?;
        let iv = f.fps_for(m.1, m.2).into_iter().find(|iv| v4l::fps_value(*iv) <= elgato::HDR_MAX_FPS + 0.5)?;
        Some((m.0, m.1, m.2, iv.0, iv.1))
    }

    fn take_snapshot(&mut self) {
        let Some(p) = &self.preview else {
            self.say(tr("No hay imagen para fotografiar"));
            return;
        };
        let g = p.shared.lock().unwrap();
        let Some(fr) = &g.frame else { return };
        let dir = pictures_dir();
        let _ = std::fs::create_dir_all(&dir);
        let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        let path = dir.join(format!("foto-{secs}.png"));
        let res = image::save_buffer(&path, &fr.rgba, fr.width as u32, fr.height as u32, image::ColorType::Rgba8);
        drop(g);
        match res {
            Ok(()) => self.say(tr_n("Foto guardada en {}", path.display())),
            Err(e) => self.say(tr_n("No se pudo guardar la foto: {}", e)),
        }
    }

    fn save_profile(&mut self) {
        let Some(cam) = &self.cam else { return };
        if let Err(e) = camera::save_profile(&cam.entry.profile_id(), &self.profile) {
            self.say(tr_n("No se pudo guardar: {}", e));
        }
    }
}

/// XDG pictures folder (from ~/.config/user-dirs.dirs), e.g. ~/Imágenes, plus "Lente".
fn pictures_dir() -> std::path::PathBuf {
    let home = std::env::var_os("HOME").map(std::path::PathBuf::from).unwrap_or_default();
    let from_env = std::env::var_os("XDG_PICTURES_DIR").map(std::path::PathBuf::from);
    let from_file = std::fs::read_to_string(home.join(".config/user-dirs.dirs")).ok().and_then(|t| {
        t.lines().find_map(|l| {
            let v = l.strip_prefix("XDG_PICTURES_DIR=")?.trim().trim_matches('"');
            Some(std::path::PathBuf::from(v.replace("$HOME", &home.to_string_lossy())))
        })
    });
    let base = from_env.or(from_file).unwrap_or_else(|| {
        let es = home.join("Imágenes");
        if es.exists() { es } else { home.join("Pictures") }
    });
    base.join("Lente")
}

/// (fourcc, width, height, interval numerator, interval denominator)
pub type Mode = (u32, u32, u32, u32, u32);

fn mode_valid(formats: &[v4l::FormatInfo], m: Mode) -> bool {
    formats.iter().any(|f| f.fourcc == m.0 && f.fps_for(m.1, m.2).contains(&(m.3, m.4)))
}

/// Default: MJPEG 1920×1080 at 30 fps (30 fps keeps HDR available, see HDR_MAX_FPS).
fn pick_format(formats: &[v4l::FormatInfo]) -> Mode {
    for p in [v4l::FMT_MJPG, v4l::FMT_YUYV, v4l::FMT_NV12] {
        if let Some(f) = formats.iter().find(|f| f.fourcc == p) {
            let size = f
                .sizes
                .iter()
                .find(|s| **s == (1920, 1080))
                .or_else(|| f.sizes.iter().filter(|s| s.0 <= 1920).max_by_key(|s| s.0 * s.1))
                .or(f.sizes.first())
                .copied()
                .unwrap_or((1280, 720));
            let ivs = f.fps_for(size.0, size.1);
            let iv = ivs
                .iter()
                .find(|iv| v4l::fps_value(**iv) <= elgato::HDR_MAX_FPS + 0.5)
                .or(ivs.first())
                .copied()
                .unwrap_or((0, 0));
            return (p, size.0, size.1, iv.0, iv.1);
        }
    }
    (0, 0, 0, 0, 0)
}

fn mode_label(m: Mode) -> String {
    let fps = v4l::fps_value((m.3, m.4));
    let res = match (m.1, m.2) {
        (3840, 2160) => "4K".to_string(),
        (2560, 1440) => "1440p".to_string(),
        (1920, 1080) => "1080p".to_string(),
        (1280, 720) => "720p".to_string(),
        (960, 540) => "540p".to_string(),
        (640, 360) => "360p".to_string(),
        (640, 480) => "480p".to_string(),
        (w, h) => format!("{w}×{h}"),
    };
    if fps > 0.0 {
        format!("{res}{fps:.0}")
    } else {
        res
    }
}

/// A system sans font close to the original's Segoe UI Variable, looked up at fixed paths (no
/// folder walking), plus a CJK font for Japanese, Korean and Chinese. Runs on a worker thread.
fn build_fonts() -> Option<egui::FontDefinitions> {
    let home = std::env::var("HOME").unwrap_or_default();
    let roots = ["/usr/share/fonts", "/run/host/fonts", "/usr/local/share/fonts"];
    let rel_regular = [
        "inter/Inter-Regular.ttf", "TTF/Inter-Regular.ttf", "noto/NotoSans-Regular.ttf", "truetype/noto/NotoSans-Regular.ttf",
        "google-noto/NotoSans-Regular.ttf", "TTF/OpenSans-Regular.ttf", "open-sans/OpenSans-Regular.ttf", "truetype/open-sans/OpenSans-Regular.ttf",
        "cantarell/Cantarell-VF.otf", "TTF/DejaVuSans.ttf", "dejavu/DejaVuSans.ttf", "truetype/dejavu/DejaVuSans.ttf",
    ];
    let rel_semibold = [
        "inter/Inter-SemiBold.ttf", "TTF/Inter-SemiBold.ttf", "noto/NotoSans-SemiBold.ttf", "truetype/noto/NotoSans-SemiBold.ttf",
        "google-noto/NotoSans-SemiBold.ttf", "TTF/OpenSans-SemiBold.ttf", "open-sans/OpenSans-SemiBold.ttf", "truetype/open-sans/OpenSans-SemiBold.ttf",
        "TTF/DejaVuSans-Bold.ttf", "dejavu/DejaVuSans-Bold.ttf", "truetype/dejavu/DejaVuSans-Bold.ttf",
    ];
    let rel_cjk = [
        "noto-cjk/NotoSansCJK-Regular.ttc", "opentype/noto/NotoSansCJK-Regular.ttc", "google-noto-cjk/NotoSansCJK-Regular.ttc",
        "OTF/NotoSansCJK-Regular.ttc", "noto-cjk/NotoSansCJKjp-Regular.otf", "wenquanyi/wqy-microhei/wqy-microhei.ttc",
        "truetype/wqy/wqy-microhei.ttc", "TTF/wqy-microhei.ttc", "droid/DroidSansFallbackFull.ttf", "truetype/droid/DroidSansFallbackFull.ttf",
    ];
    let user = format!("{home}/.local/share/fonts");
    let find = |rels: &[&str]| -> Option<Vec<u8>> {
        for r in roots.iter().copied().chain(std::iter::once(user.as_str())) {
            for rel in rels {
                if let Ok(b) = std::fs::read(format!("{r}/{rel}")) {
                    return Some(b);
                }
            }
        }
        None
    };
    let mut fonts = egui::FontDefinitions::default();
    let mut changed = false;
    if let Some(reg) = find(&rel_regular) {
        fonts.font_data.insert("ui".into(), std::sync::Arc::new(egui::FontData::from_owned(reg)));
        fonts.families.entry(egui::FontFamily::Proportional).or_default().insert(0, "ui".into());
        changed = true;
    }
    if crate::i18n::lang().needs_cjk() {
        if let Some(cjk) = find(&rel_cjk) {
            fonts.font_data.insert("cjk".into(), std::sync::Arc::new(egui::FontData::from_owned(cjk)));
            fonts.families.entry(egui::FontFamily::Proportional).or_default().push("cjk".into());
            changed = true;
        }
    }
    let fam = egui::FontFamily::Name("semibold".into());
    let mut list = Vec::new();
    if let Some(sb) = find(&rel_semibold) {
        fonts.font_data.insert("ui-semibold".into(), std::sync::Arc::new(egui::FontData::from_owned(sb)));
        list.push("ui-semibold".to_string());
        changed = true;
    }
    list.extend(fonts.families.get(&egui::FontFamily::Proportional).cloned().unwrap_or_default());
    fonts.families.insert(fam, list);
    changed.then_some(fonts)
}

fn semibold(size: f32) -> FontId {
    FontId::new(size, egui::FontFamily::Name("semibold".into()))
}

pub fn style(ctx: &egui::Context) {
    // The "semibold" family must exist from the first frame; real fonts arrive later.
    let mut fonts = egui::FontDefinitions::default();
    let prop = fonts.families.get(&egui::FontFamily::Proportional).cloned().unwrap_or_default();
    fonts.families.insert(egui::FontFamily::Name("semibold".into()), prop);
    ctx.set_fonts(fonts);
    ctx.set_visuals(egui::Visuals::dark());
    ctx.global_style_mut(|s| {
        s.spacing.item_spacing = Vec2::new(8.0, 6.0);
        s.spacing.button_padding = Vec2::new(11.0, 5.0);
        s.spacing.interact_size.y = 30.0;
        s.text_styles.insert(egui::TextStyle::Body, FontId::proportional(14.0));
        s.text_styles.insert(egui::TextStyle::Button, FontId::proportional(14.0));
        let v = &mut s.visuals;
        v.panel_fill = BG;
        v.window_fill = ELEVATED;
        v.window_stroke = Stroke::new(1.0, border());
        v.window_corner_radius = CornerRadius::same(8);
        v.menu_corner_radius = CornerRadius::same(8);
        v.extreme_bg_color = BG_ALT;
        v.faint_bg_color = CARD;
        v.selection.bg_fill = Color32::from_rgba_unmultiplied(0x60, 0xcd, 0xff, 40);
        v.selection.stroke = Stroke::new(1.0, ACCENT);
        v.hyperlink_color = ACCENT;
        for w in [&mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active, &mut v.widgets.open, &mut v.widgets.noninteractive] {
            w.corner_radius = CornerRadius::same(4);
        }
        v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, border());
        v.widgets.inactive.bg_fill = ELEVATED;
        v.widgets.inactive.weak_bg_fill = ELEVATED;
        v.widgets.inactive.bg_stroke = Stroke::new(1.0, border());
        v.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT);
        v.widgets.hovered.weak_bg_fill = Color32::from_rgb(0x32, 0x32, 0x32);
        v.widgets.hovered.bg_fill = Color32::from_rgb(0x32, 0x32, 0x32);
        v.widgets.hovered.bg_stroke = Stroke::new(1.0, Color32::from_white_alpha(36));
        v.widgets.active.weak_bg_fill = Color32::from_rgb(0x27, 0x27, 0x27);
        v.widgets.open.weak_bg_fill = Color32::from_rgb(0x27, 0x27, 0x27);
        v.override_text_color = Some(TEXT);
    });
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.poll_opening(&ctx);
        if !self.timing_reported {
            self.timing_reported = true;
            if std::env::var_os("LENTE_TIMING").is_some() {
                eprintln!("primer fotograma: {:.1} ms", self.started.elapsed().as_secs_f64() * 1000.0);
            }
        }
        self.update_texture(&ctx);
        self.check_fallback(&ctx);
        // Throttled writes (focus, zoom, pan, tilt, temperature, tint): send the newest value once
        // its 50 ms have passed, as the original's Throttler does.
        if let Some(c) = &mut self.cam {
            if c.has_pending() {
                let errs = c.pump();
                if let Some(e) = errs.into_iter().next() {
                    self.say(e);
                }
                ctx.request_repaint_after(std::time::Duration::from_millis(crate::elgato::THROTTLE_MS));
            }
        }
        if self.last_readout.elapsed().as_secs_f32() > 1.0 {
            self.last_readout = Instant::now();
            if let Some(c) = &mut self.cam {
                c.refresh_readouts();
            }
            if self.preview.is_some() {
                ctx.request_repaint_after(std::time::Duration::from_millis(500));
            }
        }

        // F11 toggles full screen, Esc leaves it (as in the original).
        let (f11, esc) = ctx.input(|i| (i.key_pressed(egui::Key::F11), i.key_pressed(egui::Key::Escape)));
        if f11 || (esc && self.fullscreen) {
            self.set_fullscreen(&ctx, !self.fullscreen && f11);
        }
        self.step_transition(&ctx);
        if self.fullscreen {
            egui::CentralPanel::default()
                .frame(egui::Frame::new().fill(Color32::BLACK).inner_margin(Margin::same(0)))
                .show(ui, |ui| self.capture_area(ui, &ctx));
            self.handle_screenshot(&ctx);
            return;
        }
        egui::Panel::top("title")
            .exact_size(48.0)
            .show_separator_line(false)
            .frame(egui::Frame::new().fill(BG_ALT).inner_margin(Margin::symmetric(16, 0)))
            .show(ui, |ui| self.title_bar(ui));
        egui::Panel::left("nav")
            .exact_size(232.0)
            .show_separator_line(false)
            .frame(egui::Frame::new().fill(SIDEBAR).inner_margin(Margin::symmetric(8, 12)))
            .show(ui, |ui| self.nav(ui, &ctx));
        if self.pane_open {
            egui::Panel::right("pane")
                .exact_size(372.0)
                .show_separator_line(false)
                .frame(egui::Frame::new().fill(BG).inner_margin(Margin::ZERO))
                .show(ui, |ui| self.pane(ui));
        }
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(BG_ALT).inner_margin(Margin::same(0)))
            .show(ui, |ui| self.capture_area(ui, &ctx));

        self.handle_screenshot(&ctx);
    }
}

// ------------------------------------------------------------------ regions
impl App {
    fn set_fullscreen(&mut self, ctx: &egui::Context, on: bool) {
        self.fullscreen = on;
        ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(on));
        if on {
            self.say(tr("Pulsa F11 o Esc para salir de pantalla completa."));
        }
    }

    /// Move zoom/pan/tilt step by step towards a frame preset.
    /// Duration = distance × 0.5 / speed (TransitionHelper::GetDuration 0x1802a0000).
    fn step_transition(&mut self, ctx: &egui::Context) {
        let Some(tr_) = &mut self.transition else { return };
        let Some(cam) = &mut self.cam else {
            self.transition = None;
            return;
        };
        ctx.request_repaint_after(std::time::Duration::from_millis(elgato::TRANSITION_STEP_MS));
        if tr_.last_step.elapsed().as_millis() < elgato::TRANSITION_STEP_MS as u128 {
            return;
        }
        tr_.last_step = Instant::now();
        tr_.step += 1;
        let done = tr_.step >= tr_.steps;
        let writes: Vec<(usize, i32)> = if done {
            tr_.target.clone()
        } else {
            // DoTransition (0x1802a3c00): 1/zoom linear, the crop's edges in straight lines.
            let f = tr_.step as f64 / tr_.steps as f64;
            let z = elgato::zoom_at(tr_.z0, tr_.z1, tr_.steps, tr_.step);
            let w = elgato::SENSOR_WIDTH / z;
            let h = elgato::SENSOR_HEIGHT / z;
            let x = tr_.from[0] + (tr_.to[0] - tr_.from[0]) * f;
            let y = tr_.from[1] + (tr_.to[1] - tr_.from[1]) * f;
            let (pan, tilt) = elgato::crop_to_pan_tilt([x, y, w, h]);
            let mut v = Vec::new();
            if let Some(i) = tr_.zoom {
                v.push((i, zoom_value(&cam.controls[i], z)));
            }
            if let Some(i) = tr_.pan {
                v.push((i, ptz_denorm(&cam.controls[i], pan)));
            }
            if let Some(i) = tr_.tilt {
                v.push((i, ptz_denorm(&cam.controls[i], tilt)));
            }
            v
        };
        for (i, v) in writes {
            if cam.controls[i].value != v {
                let _ = cam.set(i, v);
            }
        }
        if done {
            self.transition = None;
        }
    }

    fn update_texture(&mut self, ctx: &egui::Context) {
        let Some(p) = &self.preview else { return };
        let g = p.shared.lock().unwrap();
        if let Some(fr) = &g.frame {
            if fr.seq != self.last_seq {
                self.last_seq = fr.seq;
                let img = egui::ColorImage::from_rgba_unmultiplied([fr.width, fr.height], &fr.rgba);
                match &mut self.texture {
                    Some(t) => t.set(img, egui::TextureOptions::LINEAR),
                    None => self.texture = Some(ctx.load_texture("preview", img, egui::TextureOptions::LINEAR)),
                }
            }
        }
    }

    fn handle_screenshot(&mut self, ctx: &egui::Context) {
        let Some(shot) = &mut self.shot else { return };
        shot.frames += 1;
        ctx.request_repaint();
        if shot.frames >= 120 && !shot.requested {
            shot.requested = true;
            let path = shot.path.clone();
            let done = shot.done.clone();
            let c2 = ctx.clone();
            ctx.request_screenshot(move |img| {
                let [w, h] = img.size;
                let mut bytes = Vec::with_capacity(w * h * 4);
                for p in &img.pixels {
                    bytes.extend_from_slice(&p.to_array());
                }
                if let Err(e) = image::save_buffer(&path, &bytes, w as u32, h as u32, image::ColorType::Rgba8) {
                    eprintln!("screenshot failed: {e}");
                }
                done.store(true, std::sync::atomic::Ordering::SeqCst);
                c2.request_repaint();
            });
        }
        if shot.done.load(std::sync::atomic::Ordering::SeqCst) || shot.frames > 600 {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    fn title_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_centered(|ui| {
            app_logo(ui);
            ui.add_space(4.0);
            ui.label(RichText::new(tr("Lente")).font(semibold(15.0)));
            ui.label(RichText::new(tr("Captura")).size(13.0).color(TEXT_DIM));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if self.status_time.elapsed().as_secs() < 8 && !self.status.is_empty() {
                    ui.add(egui::Label::new(RichText::new(&self.status).size(12.5).color(TEXT_2)).truncate());
                }
            });
        });
    }

    fn nav(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            ui.label(RichText::new(tr("Dispositivos")).size(12.5).color(TEXT_DIM));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if icon_button(ui, Icon::Refresh, &tr("Buscar cámaras de nuevo")).clicked() {
                    self.entries = camera::discover();
                    if self.entries.is_empty() {
                        self.entries.push(camera::demo_entry());
                    }
                    self.selected = self.selected.min(self.entries.len().saturating_sub(1));
                    self.open_selected(ctx);
                    let n = self.entries.len();
                    self.say(tr_n("{} dispositivo(s) encontrados", n));
                }
            });
        });
        ui.add_space(4.0);
        let mut clicked = None;
        for (i, e) in self.entries.iter().enumerate() {
            let sel = i == self.selected;
            let (rect, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 56.0), Sense::click());
            let p = ui.painter();
            let fill = if sel { active_bg() } else if resp.hovered() { hover_bg() } else { Color32::TRANSPARENT };
            p.rect_filled(rect, CornerRadius::same(4), fill);
            if sel {
                // WinUI selection pill
                let pill = Rect::from_center_size(pos2(rect.left() + 2.0, rect.center().y), Vec2::new(3.0, 18.0));
                p.rect_filled(pill, CornerRadius::same(2), ACCENT);
            }
            device_glyph(p, Rect::from_center_size(pos2(rect.left() + 30.0, rect.center().y), Vec2::new(36.0, 36.0)), e.elgato_webcam);
            p.text(pos2(rect.left() + 56.0, rect.top() + 18.0), Align2::LEFT_CENTER, short(&e.name, 22), FontId::proportional(14.0), TEXT);
            let (sub, col) = if e.demo {
                (tr("Ejemplo sin hardware"), TEXT_DIM)
            } else if e.error.is_some() {
                (tr("Sin acceso"), WARNING)
            } else {
                (tr("Conectada"), SUCCESS)
            };
            p.circle_filled(pos2(rect.left() + 60.0, rect.top() + 38.0), 3.5, col);
            p.text(pos2(rect.left() + 68.0, rect.top() + 38.0), Align2::LEFT_CENTER, sub, FontId::proportional(12.0), TEXT_DIM);
            if resp.clicked() {
                clicked = Some(i);
            }
            ui.add_space(2.0);
        }
        if let Some(i) = clicked {
            if i != self.selected {
                self.selected = i;
                self.open_selected(ctx);
            }
        }
        ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
            ui.add_space(6.0);
            egui::Frame::new().inner_margin(Margin::symmetric(8, 0)).show(ui, |ui| {
                ui.set_max_width(ui.available_width());
                ui.add(egui::Label::new(RichText::new(tr("Los cambios llegan a la cámara al instante: también se ven en OBS, Discord o el navegador.")).size(11.5).color(TEXT_FAINT)).wrap());
            });
        });
    }

    fn capture_area(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let full = ui.max_rect();
        let footer_h = 56.0;
        let preview_rect = Rect::from_min_max(full.min, pos2(full.max.x, full.max.y - footer_h));
        let footer_rect = Rect::from_min_max(pos2(full.min.x, full.max.y - footer_h), full.max);
        ui.allocate_rect(full, Sense::hover());

        // ---- preview
        let p = ui.painter_at(preview_rect);
        p.rect_filled(preview_rect, CornerRadius::ZERO, Color32::from_rgb(0x0e, 0x0e, 0x0e));
        let aspect = self.texture.as_ref().map(|t| t.aspect_ratio()).unwrap_or(16.0 / 9.0);
        let inner = preview_rect.shrink(16.0);
        let mut size = Vec2::new(inner.width(), inner.width() / aspect);
        if size.y > inner.height() {
            size = Vec2::new(inner.height() * aspect, inner.height());
        }
        let video = Rect::from_center_size(inner.center(), size);
        if let Some(t) = &self.texture {
            let uv = if self.mirror_preview {
                Rect::from_min_max(pos2(1.0, 0.0), pos2(0.0, 1.0))
            } else {
                Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0))
            };
            p.image(t.id(), video, uv, Color32::WHITE);
        } else {
            p.rect_filled(video, CornerRadius::same(8), Color32::from_rgb(0x16, 0x16, 0x16));
            p.rect_stroke(video, CornerRadius::same(8), Stroke::new(1.0, Color32::from_white_alpha(12)), StrokeKind::Inside);
        }
        if self.grid_on && self.texture.is_some() {
            // rule-of-thirds guides (CameraHudGridLineBrush in the original)
            let s = Stroke::new(1.0, Color32::from_white_alpha(70));
            for k in 1..3 {
                let x = video.left() + video.width() * k as f32 / 3.0;
                let y = video.top() + video.height() * k as f32 / 3.0;
                p.line_segment([pos2(x, video.top()), pos2(x, video.bottom())], s);
                p.line_segment([pos2(video.left(), y), pos2(video.right(), y)], s);
            }
        }
        if self.hud_on && self.cam.is_some() && self.texture.is_some() {
            self.hud(&p, video);
        }
        if ui.interact(video, ui.scope_id().with("video"), Sense::click()).double_clicked() {
            let on = !self.fullscreen;
            self.set_fullscreen(ctx, on);
        }
        // in-use badge (VideoInUseBadge in the original)
        let busy = self.preview.as_ref().and_then(|pr| pr.shared.lock().unwrap().error.clone());
        if let Some(e) = &busy {
            if e.contains("ocupada") || e.contains("otra aplicación") {
                badge(&p, video.left_top() + Vec2::new(12.0, 12.0), &tr("Cámara en uso por otra app"), WARNING);
            }
        }
        let cam_name = self.cam.as_ref().map(|c| c.entry.name.clone()).unwrap_or_default();
        if self.texture.is_none() {
            let msg_rect = video.shrink2(Vec2::new(48.0, 56.0));
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(msg_rect).layout(Layout::top_down(Align::Center)));
            self.no_image_message(&mut child, ctx, msg_rect, busy, &cam_name);
        }

        // ---- footer command bar
        ui.painter().rect_filled(footer_rect, CornerRadius::ZERO, BG_ALT);
        let mut fui = ui.new_child(egui::UiBuilder::new().max_rect(footer_rect.shrink2(Vec2::new(16.0, 10.0))).layout(Layout::left_to_right(Align::Center)));
        self.footer(&mut fui, ctx);
    }

    fn no_image_message(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, rect: Rect, busy: Option<String>, cam_name: &str) {
        let blocked = self.open_error.is_some();
        ui.add_space((rect.height() / 2.0 - if blocked && self.sandboxed { 125.0 } else { 50.0 }).max(0.0));
        big_camera_glyph(ui);
        ui.add_space(10.0);
        if blocked && self.sandboxed {
            ui.label(RichText::new(tr("Craft Anything no deja ver la cámara")).font(semibold(18.0)));
            ui.add_space(4.0);
            ui.label(
                RichText::new(tr_n(
                    "Tu {} está conectada, pero Craft Anything abre las apps en un espacio aislado sin acceso a cámaras. Abre Lente directamente en tu sistema para ver la imagen y cambiar los ajustes de verdad.",
                    cam_name,
                ))
                .size(13.5)
                .color(TEXT_2),
            );
            ui.add_space(10.0);
            let cmd = launcher_command();
            egui::Frame::new().fill(ELEVATED).corner_radius(CornerRadius::same(4)).inner_margin(Margin::symmetric(10, 6)).stroke(Stroke::new(1.0, border())).show(ui, |ui| {
                ui.label(RichText::new(&cmd).monospace().size(12.0).color(TEXT));
            });
            ui.add_space(8.0);
            if accent_button(ui, &tr("Copiar comando")).clicked() {
                ctx.copy_text(cmd);
                self.say(tr("Comando copiado: pégalo en una terminal"));
            }
            ui.add_space(4.0);
            ui.label(RichText::new(tr("o ejecuta «abrir-lente.sh» desde la carpeta de la app")).size(12.0).color(TEXT_DIM));
        } else if blocked {
            ui.label(RichText::new(tr("No se puede abrir la cámara")).font(semibold(18.0)));
            ui.label(RichText::new(self.open_error.clone().unwrap_or_default()).size(13.5).color(TEXT_2));
        } else if let Some(e) = busy {
            ui.label(RichText::new(tr("Sin imagen")).font(semibold(18.0)));
            ui.label(RichText::new(e).size(13.5).color(TEXT_2));
        } else if self.cam.as_ref().map(|c| c.entry.demo).unwrap_or(false) {
            ui.label(RichText::new(tr("No se encontró ninguna cámara")).font(semibold(18.0)));
            ui.label(RichText::new(tr("Conecta tu cámara y pulsa el botón de actualizar.")).size(13.5).color(TEXT_2));
        } else if !self.preview_on {
            ui.label(RichText::new(tr("Vista previa en pausa")).font(semibold(18.0)));
        } else if self.preview.is_some() {
            ui.label(RichText::new(tr("Esperando imagen de la cámara…")).font(semibold(16.0)));
        } else {
            ui.label(RichText::new(tr("Esta cámara no ofrece un formato de vídeo compatible")).font(semibold(16.0)));
        }
    }

    fn hud(&self, p: &egui::Painter, video: Rect) {
        let Some(cam) = &self.cam else { return };
        let val = |id: u32| cam.find_std(id).map(|i| cam.controls[i].value);
        let key = |k: &str| cam.find(k).map(|i| cam.controls[i].value);
        let mut items: Vec<(String, bool)> = Vec::new();
        if let Some(a) = val(elgato::CID_EXPOSURE_AUTO).or_else(|| key("ae")).map(|v| v != 1) {
            items.push((if a { "AE Auto" } else { "AE Manual" }.into(), a));
        }
        if let Some(a) = val(elgato::CID_FOCUS_AUTO).or_else(|| key("af")).map(|v| v != 0) {
            items.push((if a { "AF Auto" } else { "AF Manual" }.into(), a));
        }
        if let Some(a) = val(0x0098_090c).or_else(|| key("awb")).map(|v| v != 0) {
            items.push((if a { "AWB Auto" } else { "AWB Manual" }.into(), a));
        }
        if let Some(dr) = key("dynamic_range").or_else(|| key("dr")) {
            items.push((tr(["SDR", "Amplio", "HDR"][dr.clamp(0, 2) as usize]), dr == 2));
        }
        for (l, v) in &cam.readouts {
            if let Some(v) = v {
                let txt = match l.as_str() {
                    "Ganancia actual" => tr_n("Ganancia {}", v),
                    "Temperatura actual (K)" => format!("{v} K"),
                    "Tiempo de exposición actual" => tr_n("Exp {}", v),
                    _ => continue,
                };
                items.push((txt, false));
            }
        }
        if let Some(pr) = &self.preview {
            let g = pr.shared.lock().unwrap();
            if let Some((w, h, _)) = g.actual {
                items.push((format!("{w}×{h} · {:.0} fps", if g.camera_fps > 0.0 { g.camera_fps } else { g.fps }), false));
            }
        } else if self.fmt_choice.0 != 0 {
            items.push((mode_label(self.fmt_choice), false));
        }
        if items.is_empty() {
            return;
        }
        let font = FontId::proportional(12.0);
        let widths: Vec<f32> = items.iter().map(|(t, _)| p.layout_no_wrap(t.clone(), font.clone(), TEXT).size().x + 16.0).collect();
        let total: f32 = widths.iter().sum::<f32>() + 6.0 * (items.len() - 1) as f32 + 16.0;
        let bar = Rect::from_center_size(pos2(video.center().x, video.bottom() - 26.0), Vec2::new(total.min(video.width() - 16.0), 32.0));
        p.rect_filled(bar, CornerRadius::same(8), Color32::from_black_alpha(170));
        let mut x = bar.left() + 8.0;
        for ((t, on), w) in items.iter().zip(widths) {
            let r = Rect::from_min_size(pos2(x, bar.top() + 5.0), Vec2::new(w, 22.0));
            if r.right() > bar.right() {
                break;
            }
            p.rect_filled(r, CornerRadius::same(4), if *on { Color32::from_rgba_unmultiplied(0x60, 0xcd, 0xff, 40) } else { Color32::from_white_alpha(14) });
            p.text(r.center(), Align2::CENTER_CENTER, t, font.clone(), if *on { ACCENT } else { TEXT });
            x += w + 6.0;
        }
    }

    fn footer(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let mut restart = false;
        let (ic, tip) = if self.preview_on { (Icon::Pause, tr("Pausar vista previa")) } else { (Icon::Play, tr("Ver vista previa")) };
        if icon_button_sized(ui, ic, &tip, 36.0).clicked() {
            self.preview_on = !self.preview_on;
            restart = true;
        }
        if icon_button_sized(ui, Icon::Camera, &tr("Tomar foto (se guarda en Imágenes/Lente)"), 36.0).clicked() {
            self.take_snapshot();
        }
        if icon_button_sized(ui, Icon::Folder, &tr("Abrir carpeta de fotos"), 36.0).clicked() {
            let dir = pictures_dir();
            let _ = std::fs::create_dir_all(&dir);
            ctx.open_url(egui::OpenUrl::new_tab(format!("file://{}", dir.display())));
        }
        ui.add_space(8.0);
        let mut picked: Option<Mode> = None;
        if let Some(cam) = &self.cam {
            let cur = self.fmt_choice;
            let text = if cur.0 == 0 { tr("Formato") } else { format!("{} · {}", mode_label(cur), fmt_name(cur.0)) };
            egui::ComboBox::from_id_salt("fmt").selected_text(text).width(170.0).height(420.0).show_ui(ui, |ui| {
                for f in &cam.formats {
                    ui.label(RichText::new(fmt_name(f.fourcc)).size(12.0).color(TEXT_DIM));
                    for (s, ivs) in f.sizes.iter().zip(&f.intervals) {
                        for iv in ivs {
                            let v = (f.fourcc, s.0, s.1, iv.0, iv.1);
                            if ui.selectable_label(cur == v, mode_label(v)).clicked() {
                                picked = Some(v);
                            }
                        }
                    }
                }
            });
        }
        if let Some(m) = picked {
            self.apply_mode(m, ctx);
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if icon_button_toggle(ui, Icon::Pane, &tr("Ajustes del dispositivo"), self.pane_open).clicked() {
                self.pane_open = !self.pane_open;
            }
            if icon_button_toggle(ui, Icon::Hud, &tr("Indicadores sobre la imagen"), self.hud_on).clicked() {
                self.hud_on = !self.hud_on;
            }
            if icon_button_toggle(ui, Icon::Fullscreen, &tr("Pantalla completa (F11)"), self.fullscreen).clicked() {
                let on = !self.fullscreen;
                self.set_fullscreen(ctx, on);
            }
            if icon_button_toggle(ui, Icon::Grid, &tr("Cuadrícula"), self.grid_on).clicked() {
                self.grid_on = !self.grid_on;
            }
            if icon_button_toggle(ui, Icon::Mirror, &tr("Espejo en la vista previa"), self.mirror_preview).clicked() {
                self.mirror_preview = !self.mirror_preview;
            }
        });
        if restart {
            self.restart_preview(ctx);
        }
    }

    // ---------------------------------------------------------- settings pane
    fn pane(&mut self, ui: &mut egui::Ui) {
        let full = ui.max_rect();
        let footer_h = 64.0;
        let body = Rect::from_min_max(full.min, pos2(full.max.x, full.max.y - footer_h));
        let foot = Rect::from_min_max(pos2(full.min.x, full.max.y - footer_h), full.max);
        ui.allocate_rect(full, Sense::hover());
        ui.painter().vline(full.left(), full.y_range(), Stroke::new(1.0, Color32::from_white_alpha(15)));

        let mut b = ui.new_child(egui::UiBuilder::new().max_rect(body.shrink2(Vec2::new(16.0, 14.0))).layout(Layout::top_down(Align::Min)));
        self.pane_body(&mut b);

        ui.painter().rect_filled(foot, CornerRadius::ZERO, BG_ALT);
        ui.painter().hline(foot.x_range(), foot.top(), Stroke::new(1.0, Color32::from_white_alpha(15)));
        let mut f = ui.new_child(egui::UiBuilder::new().max_rect(foot.shrink2(Vec2::new(16.0, 14.0))).layout(Layout::right_to_left(Align::Center)));
        self.pane_footer(&mut f);
    }

    fn pane_body(&mut self, ui: &mut egui::Ui) {
        // header: device image + name (CapturePage_DeviceHeader)
        if let Some(cam) = &self.cam {
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(Vec2::new(48.0, 48.0), Sense::hover());
                device_glyph(ui.painter(), r, cam.entry.elgato_webcam);
                ui.vertical(|ui| {
                    ui.add_space(4.0);
                    ui.label(RichText::new(&cam.entry.name).font(semibold(16.0)));
                    let sub = if cam.dev.is_some() {
                        format!("{} · USB {:04x}:{:04x}", cam.entry.path, cam.entry.vendor, cam.entry.product)
                    } else {
                        tr("Sin conexión: ajustes de ejemplo")
                    };
                    ui.label(RichText::new(sub).size(12.0).color(TEXT_DIM));
                });
            });
        } else {
            ui.label(RichText::new(tr("Elige un dispositivo")).font(semibold(16.0)));
            return;
        }
        ui.add_space(10.0);
        // tabs (CapturePage_DeviceSettingsTabs)
        ui.horizontal(|ui| {
            for (t, label) in [(Tab::Camera, "Cámara"), (Tab::Device, "Dispositivo")] {
                if tab_button(ui, label, self.tab == t).clicked() {
                    self.tab = t;
                }
            }
        });
        ui.add_space(8.0);
        let ctx = ui.ctx().clone();
        match self.tab {
            Tab::Camera => self.camera_tab(ui, &ctx),
            Tab::Device => self.device_tab(ui, &ctx),
        }
    }

    fn camera_tab(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let offline = self.offline();
        let has_focus: Vec<bool> = (0..elgato::FOCUS_PRESET_COUNT).map(|s| self.profile.focus_presets.get(s).map(|p| p.is_some()).unwrap_or(false)).collect();
        let mut focus_action: Option<(usize, bool)> = None;
        let has_presets: Vec<bool> = (0..elgato::FRAME_PRESET_COUNT).map(|s| self.profile.frame_presets.get(s).map(|p| p.is_some()).unwrap_or(false)).collect();
        let cur_mode = self.fmt_choice;
        let (cam_fps, view_fps) = self.preview.as_ref().map(|p| {
            let g = p.shared.lock().unwrap();
            (g.camera_fps, g.fps)
        }).unwrap_or((0.0, 0.0));
        let Some(cam) = &mut self.cam else { return };
        let mut changes: Vec<(usize, i32)> = Vec::new();
        let mut preset_action: Option<(usize, bool)> = None;
        let mut picked_mode: Option<Mode> = None;
        let mut command: Option<camera::Command> = None;
        let open_groups = &mut self.open_groups;
        let commands = cam.commands.clone();
        let can = |c: camera::Command| commands.iter().any(|x| x.0 == c && x.1.is_some());
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            if offline {
                egui::Frame::new()
                    .fill(Color32::from_rgba_unmultiplied(0xf5, 0x9e, 0x0b, 24))
                    .stroke(Stroke::new(1.0, Color32::from_rgba_unmultiplied(0xf5, 0x9e, 0x0b, 90)))
                    .corner_radius(CornerRadius::same(4))
                    .inner_margin(Margin::same(10))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.label(RichText::new(tr("Sin acceso a la cámara: estos son los ajustes que verás al abrir Lente fuera de Craft Anything.")).size(12.5).color(TEXT));
                    });
                ui.add_space(8.0);
            }
            // Video format (Main_DeviceSettings_CameraFormatTitle in the original)
            card(ui, |ui| {
                ui.label(RichText::new(tr("Formato de vídeo")).font(semibold(14.0)));
                if let Some(m) = format_picker(ui, &cam.formats, cur_mode) {
                    picked_mode = Some(m);
                }
                // Exposure priority: with it on, the camera lengthens the exposure in low light
                // and drops below the chosen frame rate.
                if let Some(i) = cam.find_std(elgato::CID_EXPOSURE_PRIORITY) {
                    let fixed = cam.controls[i].value == 0;
                    split_row(
                        ui,
                        |ui| {
                            ui.add(egui::Label::new(RichText::new(tr("Fotogramas fijos")).size(12.5).color(TEXT_2)).truncate()).on_hover_text(tr("Que la cámara no baje los fps cuando hay poca luz (la imagen puede verse más oscura)"));
                        },
                        |ui| {
                            let mut on = fixed;
                            if toggle(ui, &mut on).clicked() {
                                changes.push((i, if on { 0 } else { 1 }));
                            }
                        },
                    );
                }
                let target = v4l::fps_value((cur_mode.3, cur_mode.4));
                if let Some(st) = cam.sensor.filter(|st| st.streaming) {
                    ui.label(RichText::new(trf("La cámara dice: {}×{} a {} fps", &[&st.width, &st.height, &st.fps])).size(12.0).color(TEXT_2))
                        .on_hover_text(tr("Estado que informa la propia cámara (igual que el programa de Elgato)"));
                }
                if cam_fps > 0.0 {
                    ui.horizontal(|ui| {
                        let ok = cam_fps >= target - 2.0;
                        ui.label(RichText::new(tr_n("Cámara: {} fps", format!("{cam_fps:.0}"))).size(12.0).color(if ok { SUCCESS } else { WARNING }));
                        ui.label(RichText::new(tr_n("· Vista previa: {} fps", format!("{view_fps:.0}"))).size(12.0).color(TEXT_DIM));
                    });
                    if cam_fps < target - 2.0 {
                        let prio = cam.find_std(elgato::CID_EXPOSURE_PRIORITY).map(|i| cam.controls[i].value != 0).unwrap_or(false);
                        let why = if prio {
                            tr("Hay poca luz: la cámara alarga la exposición y baja los fps. Activa «Fotogramas fijos» o ilumina más.")
                        } else {
                            tr("La cámara entrega menos fps de los pedidos: prueba la exposición automática, más luz o un puerto USB 3.")
                        };
                        ui.label(RichText::new(why).size(11.5).color(WARNING));
                    } else if view_fps < cam_fps - 5.0 {
                        ui.label(RichText::new(tr("La cámara va bien; es la vista previa la que no alcanza a dibujar todos los fotogramas en este equipo. OBS y las demás apps reciben los fps completos.")).size(11.5).color(TEXT_DIM));
                    }
                }
                ui.label(RichText::new(tr("Afecta a la vista previa de Lente; OBS y el navegador eligen el suyo.")).size(11.5).color(TEXT_DIM));
            });
            ui.add_space(6.0);
            for (gi, group) in Group::ALL.iter().enumerate() {
                let idxs: Vec<usize> = cam.controls.iter().enumerate().filter(|(_, c)| c.group == *group).map(|(i, _)| i).collect();
                if idxs.is_empty() {
                    continue;
                }
                let open = open_groups[gi];
                if expander_header(ui, group.title(), open).clicked() {
                    open_groups[gi] = !open;
                }
                if open_groups[gi] {
                    egui::Frame::new()
                        .fill(Color32::from_rgb(0x27, 0x27, 0x27))
                        .stroke(Stroke::new(1.0, Color32::from_white_alpha(15)))
                        .corner_radius(CornerRadius { nw: 0, ne: 0, sw: 4, se: 4 })
                        .inner_margin(Margin::symmetric(14, 10))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            for i in idxs {
                                if let Some(v) = control_row(ui, &cam.controls[i]) {
                                    changes.push((i, v));
                                }
                            }
                            if *group == Group::Frame {
                                ui.add_space(4.0);
                                ui.label(RichText::new(tr("Encuadres guardados")).size(13.0).color(TEXT_2));
                                ui.horizontal(|ui| {
                                    for (slot, has) in has_presets.iter().enumerate() {
                                        let r = preset_chip(ui, slot + 1, *has).on_hover_text(tr("Clic: aplicar · clic derecho: guardar zoom/pan/tilt actuales"));
                                        if r.clicked() {
                                            preset_action = Some((slot, !*has));
                                        }
                                        if r.secondary_clicked() {
                                            preset_action = Some((slot, true));
                                        }
                                    }
                                });
                            }
                            if *group == Group::Focus {
                                ui.add_space(4.0);
                                ui.label(RichText::new(tr("Preajustes de enfoque")).size(13.0).color(TEXT_2));
                                ui.horizontal(|ui| {
                                    for (slot, has) in has_focus.iter().enumerate() {
                                        let r = preset_chip(ui, slot + 1, *has).on_hover_text(tr("Clic: aplicar · clic derecho: guardar el enfoque actual"));
                                        if r.clicked() {
                                            focus_action = Some((slot, !*has));
                                        }
                                        if r.secondary_clicked() {
                                            focus_action = Some((slot, true));
                                        }
                                    }
                                });
                            }
                            if *group == Group::Focus && (can(camera::Command::Refocus) || offline) {
                                ui.add_space(4.0);
                                if ui.add(egui::Button::new(tr("Reenfocar ahora")).min_size(Vec2::new(0.0, 30.0))).on_hover_text(tr("Enfoca una vez y se queda ahí (como el botón de reenfoque del original)")).clicked() {
                                    command = Some(camera::Command::Refocus);
                                }
                            }
                            if *group == Group::WhiteBalance && (can(camera::Command::AutoWhiteBalanceOnce) || offline) {
                                ui.add_space(4.0);
                                if ui.add(egui::Button::new(tr("Ajustar balance una vez")).min_size(Vec2::new(0.0, 30.0))).clicked() {
                                    command = Some(camera::Command::AutoWhiteBalanceOnce);
                                }
                            }
                            if *group == Group::Exposure {
                                if can(camera::Command::AutoExposureOnce) || offline {
                                    ui.add_space(4.0);
                                    if ui.add(egui::Button::new(tr("Ajustar exposición una vez")).min_size(Vec2::new(0.0, 30.0))).clicked() {
                                        command = Some(camera::Command::AutoExposureOnce);
                                    }
                                }
                                if cam.find_std(elgato::CID_BRIGHTNESS).is_some() {
                                    ui.label(RichText::new(tr("La compensación de exposición sólo funciona con la exposición automática activa.")).size(11.5).color(TEXT_DIM));
                                }
                            }
                        });
                }
                ui.add_space(6.0);
            }
        });
        let mut msgs = Vec::new();
        let mut hdr_switch = None;
        for (i, v) in changes {
            if cam.controls[i].key == "dynamic_range" && v == 2 {
                // The original refuses HDR above 30 fps (FUN_1802cf390). Lente lowers the frame
                // rate to 30 for you instead of refusing.
                let fps = v4l::fps_value((cur_mode.3, cur_mode.4));
                if fps > elgato::HDR_MAX_FPS + 0.5 {
                    hdr_switch = Some(i);
                    continue;
                }
            }
            if let Err(e) = cam.set(i, v) {
                msgs.push(e);
            }
        }
        if let Some(c) = command {
            match cam.run_command(c) {
                Ok(m) | Err(m) => msgs.push(m),
            }
        }
        for m in msgs {
            self.say(m);
        }
        if let Some(i) = hdr_switch {
            if let Some(m) = self.hdr_mode() {
                self.apply_mode(m, ctx);
                if let Some(cam) = &mut self.cam {
                    match cam.set(i, 2) {
                        Ok(()) => self.say(tr_n("HDR activado: la cámara pasó a {} (el HDR funciona hasta 30 fps)", mode_label(m))),
                        Err(e) => self.say(e),
                    }
                }
            } else {
                self.say(tr("El HDR necesita 30 fps o menos y este formato no los ofrece"));
            }
        }
        if let Some(m) = picked_mode {
            self.apply_mode(m, ctx);
        }
        if let Some((slot, save)) = preset_action {
            self.frame_preset(slot, save);
        }
        if let Some((slot, save)) = focus_action {
            self.focus_preset(slot, save);
        }
    }

    fn frame_preset(&mut self, slot: usize, save: bool) {
        self.profile.frame_presets.resize(elgato::FRAME_PRESET_COUNT, None);
        let Some(cam) = &mut self.cam else { return };
        let keys: Vec<String> = [elgato::CID_ZOOM, elgato::CID_PAN, elgato::CID_TILT]
            .iter()
            .filter_map(|id| cam.find_std(*id).map(|i| cam.controls[i].key.clone()))
            .chain(["zoom", "pan", "tilt"].iter().filter_map(|k| cam.find(k).map(|i| cam.controls[i].key.clone())))
            .collect();
        if save {
            let values = keys.iter().filter_map(|k| cam.find(k).map(|i| (k.clone(), cam.controls[i].value))).collect();
            let mut name = tr_n("Encuadre {}", slot + 1);
            name.truncate(elgato::FRAME_PRESET_NAME_MAX_LEN);
            self.profile.frame_presets[slot] = Some(camera::FramePreset { name, values });
            self.save_profile();
            self.say(tr_n("Encuadre {} guardado", slot + 1));
        } else if let Some(p) = self.profile.frame_presets[slot].clone() {
            if !self.profile.smooth_transitions || cam.dev.is_none() {
                let errs = cam.apply_values(&p.values);
                self.say(if errs.is_empty() { tr_n("Encuadre {} aplicado", slot + 1) } else { errs.join("; ") });
                return;
            }
            let mut target = Vec::new();
            for (k, v) in &p.values {
                if let Some(i) = cam.find(k) {
                    let c = &cam.controls[i];
                    target.push((i, (*v).clamp(c.min.min(c.max), c.max.max(c.min))));
                }
            }
            let pan = cam.find_std(elgato::CID_PAN);
            let tilt = cam.find_std(elgato::CID_TILT);
            let zoom = cam.find_std(elgato::CID_ZOOM);
            let goal = |idx: Option<usize>| idx.map(|i| target.iter().find(|t| t.0 == i).map(|t| t.1).unwrap_or(cam.controls[i].value));
            let now = |idx: Option<usize>| idx.map(|i| cam.controls[i].value);
            let norm = |idx: Option<usize>, v: Option<i32>| match (idx, v) {
                (Some(i), Some(v)) => ptz_norm(&cam.controls[i], v),
                _ => 0.0,
            };
            let ratio = |v: Option<i32>| match (zoom, v) {
                (Some(i), Some(v)) => zoom_ratio(&cam.controls[i], v),
                _ => 1.0,
            };
            let (z0, z1) = (ratio(now(zoom)), ratio(goal(zoom)));
            let from = elgato::crop_rect(norm(pan, now(pan)), norm(tilt, now(tilt)), z0);
            let to = elgato::crop_rect(norm(pan, goal(pan)), norm(tilt, goal(tilt)), z1);
            let steps = elgato::transition_steps(elgato::crop_distance(from, to));
            self.transition = Some(Transition {
                pan,
                tilt,
                zoom,
                from,
                to,
                z0,
                z1,
                steps,
                step: 0,
                target,
                last_step: Instant::now() - std::time::Duration::from_secs(1),
            });
            self.say(tr_n("Encuadre {} aplicado", slot + 1));
        }
    }

    fn focus_preset(&mut self, slot: usize, save: bool) {
        self.profile.focus_presets.resize(elgato::FOCUS_PRESET_COUNT, None);
        let Some(cam) = &mut self.cam else { return };
        let fi = cam.find_std(elgato::CID_FOCUS_ABS).or_else(|| cam.find("focus"));
        let ai = cam.find_std(elgato::CID_FOCUS_AUTO).or_else(|| cam.find("af"));
        let Some(fi) = fi else { return };
        if save {
            self.profile.focus_presets[slot] = Some(cam.controls[fi].value);
            self.save_profile();
            self.say(tr_n("Enfoque {} guardado", slot + 1));
        } else if let Some(v) = self.profile.focus_presets[slot] {
            // The original only writes the focus distance (0x1802ac850); Linux's driver refuses
            // that while autofocus is on, so autofocus is switched off first.
            if let Some(ai) = ai {
                let _ = cam.set(ai, 0);
            }
            match cam.set(fi, v) {
                Ok(()) => self.say(tr_n("Enfoque {} aplicado", slot + 1)),
                Err(e) => self.say(e),
            }
        }
    }

    fn device_tab(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let sandboxed = self.sandboxed;
        let Some(cam) = &mut self.cam else { return };
        let mut apply = self.profile.apply_on_open;
        let mut use_cam_profile = self.profile.use_camera_profile;
        let mut lang_pick: Option<crate::i18n::Lang> = None;
        let mut smooth = self.profile.smooth_transitions;
        let mut use_win = false;
        let mut msgs: Vec<String> = Vec::new();
        let mut led_edit = self.led_edit;
        let mut led_apply = false;
        let mut ask: Option<camera::Command> = None;
        let mut run: Option<camera::Command> = None;
        let confirm = self.confirm;
        let commands = cam.commands.clone();
        let can = |c: camera::Command| commands.iter().any(|x| x.0 == c && x.1.is_some());
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            // ---- USB permission (needed for the controls Linux's driver hides)
            if cam.usb_error.is_some() && cam.entry.elgato_webcam {
                egui::Frame::new()
                    .fill(Color32::from_rgba_unmultiplied(0x60, 0xcd, 0xff, 18))
                    .stroke(Stroke::new(1.0, Color32::from_rgba_unmultiplied(0x60, 0xcd, 0xff, 90)))
                    .corner_radius(CornerRadius::same(4))
                    .inner_margin(Margin::same(12))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.label(RichText::new(tr("Activa el resto de funciones")).font(semibold(14.0)));
                        let n = cam.features.iter().filter(|f| f.status == camera::Status::NeedsPermission).count();
                        ui.label(
                            RichText::new(tr_n(
                                "{} funciones (tono, intensidad, bloqueos, reenfoque, reinicio…) necesitan hablar con la cámara por USB. Linux pide un permiso una sola vez; después funcionan aunque la cámara esté en uso.",
                                n,
                            ))
                            .size(12.5)
                            .color(TEXT_2),
                        );
                        ui.add_space(6.0);
                        ui.horizontal_wrapped(|ui| {
                            if !sandboxed && accent_button(ui, &tr("Dar permiso (pide tu contraseña)")).clicked() {
                                let out = std::process::Command::new("pkexec").args(["sh", "-c", &crate::usb::install_command()]).status();
                                msgs.push(match out {
                                    Ok(s) if s.success() => tr("Permiso instalado: desconecta y vuelve a conectar la cámara, y pulsa actualizar"),
                                    Ok(_) => tr("No se instaló el permiso (cancelado)"),
                                    Err(e) => tr_n("No se pudo pedir el permiso: {}", e),
                                });
                            }
                            if ui.add(egui::Button::new(tr("Copiar comando")).min_size(Vec2::new(0.0, 32.0))).on_hover_text(tr("Para pegarlo en una terminal")).clicked() {
                                ctx.copy_text(format!("sudo sh -c \"{}\"", crate::usb::install_command().replace('"', "\\\"")));
                                msgs.push(tr("Comando copiado: pégalo en una terminal"));
                            }
                        });
                        ui.label(RichText::new(tr_n("Instala {} (también está en la carpeta de la app).", crate::usb::RULE_PATH)).size(11.0).color(TEXT_FAINT));
                    });
                ui.add_space(6.0);
            }
            card(ui, |ui| {
                split_row(
                    ui,
                    |ui| {
                        ui.add(egui::Label::new(tr("Aplicar mis ajustes al abrir")).truncate());
                    },
                    |ui| {
                        toggle(ui, &mut apply);
                    },
                );
                ui.label(RichText::new(tr("Además de guardarlos en la cámara, Lente los vuelve a aplicar al abrirla.")).size(11.5).color(TEXT_DIM));
            });
            ui.add_space(6.0);
            // ---- language (the 14 languages of the official app)
            card(ui, |ui| {
                split_row(
                    ui,
                    |ui| {
                        ui.add(egui::Label::new(RichText::new(tr("Idioma")).font(semibold(14.0))).truncate());
                    },
                    |ui| {
                        let cur = crate::i18n::lang();
                        egui::ComboBox::from_id_salt("lang").selected_text(cur.native()).width(170.0).show_ui(ui, |ui| {
                            for l in crate::i18n::Lang::ALL {
                                if ui.selectable_label(l == cur, l.native()).clicked() && l != cur {
                                    lang_pick = Some(l);
                                }
                            }
                    },
                );
                });
                split_row(
                    ui,
                    |ui| {
                        ui.add(egui::Label::new(RichText::new(tr("Transición suave entre encuadres")).size(12.5)).truncate());
                    },
                    |ui| {
                        toggle(ui, &mut smooth);
                    },
                );
            });
            ui.add_space(6.0);
            // ---- settings stored in the camera (what Windows saved)
            card(ui, |ui| {
                ui.label(RichText::new(tr("Ajustes guardados en la cámara")).font(semibold(14.0)));
                ui.label(RichText::new(tr("La cámara guarda dentro el último perfil de la app de Windows. Usarlo hace que la imagen se vea igual que allí.")).size(11.5).color(TEXT_DIM));
                match &cam.camera_profile {
                    None => {
                        let t = tr(if cam.dev.is_none() { "Se lee al abrir la cámara." } else { "Esta cámara no tiene un perfil legible." });
                        ui.label(RichText::new(t).size(12.0).color(TEXT_DIM));
                    }
                    Some(rows) => {
                        let diff = rows.iter().filter(|r| r.target.as_ref().map(|t| Some(t.1) != r.now).unwrap_or(false)).count();
                        ui.add_space(4.0);
                        egui::Grid::new("winprof").num_columns(3).spacing([10.0, 3.0]).show(ui, |ui| {
                            ui.label(RichText::new(tr("Ajuste")).size(11.5).color(TEXT_DIM));
                            ui.label(RichText::new(tr("En la cámara")).size(11.5).color(TEXT_DIM));
                            ui.label(RichText::new(tr("Ahora")).size(11.5).color(TEXT_DIM));
                            ui.end_row();
                            for r in rows {
                                let differs = r.target.as_ref().map(|t| Some(t.1) != r.now).unwrap_or(false);
                                let col = if differs { WARNING } else { TEXT_2 };
                                ui.add(egui::Label::new(RichText::new(tr(r.label)).size(12.0)).truncate());
                                ui.label(RichText::new(r.stored.to_string()).size(12.0).color(col));
                                ui.label(RichText::new(r.now.map(|v| v.to_string()).unwrap_or("—".into())).size(12.0).color(col));
                                ui.end_row();
                            }
                        });
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            if accent_button(ui, &if diff > 0 { tr_n("Usar los ajustes de Windows ({} distintos)", diff) } else { tr("Usar los ajustes de Windows") }).clicked() {
                                use_win = true;
                            }
                        });
                    }
                }
                split_row(
                    ui,
                    |ui| {
                        ui.add(egui::Label::new(RichText::new(tr("Aplicarlos al abrir la cámara")).size(12.5)).truncate());
                    },
                    |ui| {
                        toggle(ui, &mut use_cam_profile);
                    },
                );
            });
            ui.add_space(6.0);
            // ---- status LED
            if let Some(led) = &mut led_edit {
                card(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(tr("Luz de estado")).font(semibold(14.0)));
                        ui.label(RichText::new(tr("experimental")).size(11.0).color(WARNING));
                    });
                    let items = [(0, "Apagada"), (1, "Encendida al transmitir"), (2, "Siempre encendida")];
                    let cur = tr(items.iter().find(|i| i.0 == led.mode).map(|i| i.1).unwrap_or("—"));
                    egui::ComboBox::from_id_salt("led_mode").selected_text(cur).width(ui.available_width() - 2.0).show_ui(ui, |ui| {
                        for (v, l) in items {
                            if ui.selectable_label(led.mode == v, tr(l)).clicked() && led.mode != v {
                                led.mode = v;
                                led_apply = true;
                            }
                        }
                    });
                    split_row(
                        ui,
                        |ui| {
                            ui.add(egui::Label::new(tr("Brillo")).truncate());
                        },
                        |ui| {
                            ui.label(RichText::new(format!("{} %", led.percent)).size(13.0).color(TEXT_2));
                        },
                    );
                    if let Some(v) = win_slider(ui, led.percent, 0, 100, 1) {
                        led.percent = v;
                        led_apply = true;
                    }
                });
                ui.add_space(6.0);
            }
            // ---- device commands
            card(ui, |ui| {
                ui.label(RichText::new(tr("Cámara")).font(semibold(14.0)));
                for (c, label, hint) in [
                    (camera::Command::SaveToCamera, "Guardar ajustes en la cámara", "Se conservan en otros equipos"),
                    (camera::Command::FactoryReset, "Restablecer de fábrica", "Carga y guarda el perfil de fábrica"),
                    (camera::Command::Reboot, "Reiniciar la cámara", "La cámara se desconecta unos segundos"),
                ] {
                    ui.add_enabled_ui(can(c) || cam.dev.is_none() || (c == camera::Command::SaveToCamera && cam.entry.elgato_webcam), |ui| {
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.set_max_width(ui.available_width() - 100.0);
                                ui.add(egui::Label::new(tr(label)).truncate());
                                ui.add(egui::Label::new(RichText::new(tr(hint)).size(11.0).color(TEXT_DIM)).wrap());
                            });
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if confirm == Some(c) && c != camera::Command::SaveToCamera {
                                    if ui.button(tr("Cancelar")).clicked() {
                                        ask = None;
                                        run = None;
                                    }
                                    if accent_button(ui, &tr("Confirmar")).clicked() {
                                        run = Some(c);
                                    }
                                } else if ui.button(tr(if c == camera::Command::SaveToCamera { "Guardar" } else { "Hacer…" })).clicked() {
                                    if c == camera::Command::SaveToCamera {
                                        run = Some(c);
                                    } else {
                                        ask = Some(c);
                                    }
                                }
                            });
                        });
                    });
                    ui.add_space(2.0);
                }
            });
            ui.add_space(6.0);
            // ---- full support table
            card(ui, |ui| {
                ui.label(RichText::new(tr("Qué funciona en Linux")).font(semibold(14.0)));
                ui.label(RichText::new(tr("Todas las funciones de cámara del programa original y cómo las hace Lente con esta cámara.")).size(11.5).color(TEXT_DIM));
                ui.add_space(4.0);
                let mut section = "";
                for f in &cam.features {
                    if f.section != section {
                        section = f.section;
                        ui.add_space(4.0);
                        ui.label(RichText::new(tr(section)).size(12.0).color(TEXT_DIM));
                    }
                    split_row(ui, |ui| { ui.add(egui::Label::new(RichText::new(tr(f.name)).size(12.5)).truncate()); }, |ui| status_chip(ui, &f.status));
                    let note = match &f.status {
                        camera::Status::WindowsOnly(why) => tr(why),
                        _ => tr(&f.note),
                    };
                    if !note.is_empty() {
                        ui.label(RichText::new(note).size(10.5).color(TEXT_FAINT));
                    }
                }
                ui.add_space(6.0);
                ui.label(RichText::new(tr("«Linux» = por el controlador del sistema · «USB directo» = Lente habla con la cámara como lo hace el programa de Elgato.")).size(10.5).color(TEXT_FAINT));
            });
            ui.add_space(6.0);
            card(ui, |ui| {
                ui.label(RichText::new(tr("Información")).font(semibold(14.0)));
                let rows = [
                    ("Dispositivo", cam.entry.path.clone()),
                    ("USB", format!("{:04x}:{:04x}", cam.entry.vendor, cam.entry.product)),
                    ("Número de serie", if cam.entry.serial.is_empty() { "—".into() } else { cam.entry.serial.clone() }),
                    ("Acceso USB directo", if cam.usb.is_some() { tr("sí") } else if cam.usb_error.is_some() { tr("falta permiso") } else { "—".into() }),
                    (
                        "Unidades Elgato",
                        if cam.entry.xu_units.is_empty() {
                            "—".into()
                        } else {
                            cam.entry.xu_units.iter().map(|u| trf("{} ({} órdenes)", &[&u.id, &u.selectors.len()])).collect::<Vec<_>>().join(", ")
                        },
                    ),
                ];
                for (k, v) in rows {
                    split_row(
                        ui,
                        |ui| {
                            ui.add(egui::Label::new(RichText::new(tr(k)).size(12.5).color(TEXT_DIM)).truncate());
                        },
                        |ui| {
                            ui.label(RichText::new(v).size(12.5));
                        },
                    );
                }
            });
        });
        if led_apply {
            if let Some(l) = led_edit {
                if cam.dev.is_some() {
                    match cam.set_led(l) {
                        Ok(()) => msgs.push(tr("Luz de estado actualizada")),
                        Err(e) => msgs.push(tr_n("Luz de estado: {}", e)),
                    }
                }
            }
        }
        if use_win {
            let errs = cam.apply_camera_profile();
            msgs.push(if errs.is_empty() { tr("Ajustes de Windows aplicados") } else { errs.join("; ") });
        }
        if let Some(c) = run {
            if cam.dev.is_some() {
                match cam.run_command(c) {
                    Ok(m) | Err(m) => msgs.push(m),
                }
            }
            self.confirm = None;
        } else if ask.is_some() {
            self.confirm = ask;
        }
        self.led_edit = led_edit;
        for m in msgs {
            self.say(m);
        }
        if apply != self.profile.apply_on_open || use_cam_profile != self.profile.use_camera_profile || smooth != self.profile.smooth_transitions {
            self.profile.apply_on_open = apply;
            self.profile.use_camera_profile = use_cam_profile;
            self.profile.smooth_transitions = smooth;
            self.save_profile();
        }
        if let Some(l) = lang_pick {
            crate::i18n::set_lang(l);
            let fctx = ctx.clone();
            std::thread::spawn(move || {
                if let Some(fonts) = build_fonts() {
                    fctx.set_fonts(fonts);
                }
                fctx.request_repaint();
            });
        }
    }

    fn pane_footer(&mut self, ui: &mut egui::Ui) {
        // laid out right to left
        let Some(cam) = &mut self.cam else { return };
        let mut msg = None;
        let mut save = false;
        if accent_button(ui, &tr("Guardar como predeterminado"))
            .on_hover_text(tr("Guarda los ajustes dentro de la cámara (como el original) y en Lente"))
            .clicked()
        {
            self.profile.defaults = Some(cam.snapshot_values());
            self.profile.apply_on_open = true;
            save = true;
            // Like the original: the settings go into the camera's own memory, so they stay
            // with it on any computer or app. Always tried on Elgato cameras, and checked.
            if cam.dev.is_some() && cam.entry.elgato_webcam {
                msg = Some(match cam.save_to_camera() {
                    Ok(m) => m,
                    Err(e) => tr_n("Guardado en Lente; en la cámara no: {}", e),
                });
            }
        }
        if ui.add(egui::Button::new(tr("Restablecer")).min_size(Vec2::new(0.0, 32.0))).on_hover_text(tr("Volver a los valores de fábrica de cada ajuste")).clicked() {
            let errs = cam.reset_all();
            msg = Some(if errs.is_empty() { tr("Valores de fábrica restablecidos") } else { errs.join("; ") });
        }
        if save {
            self.save_profile();
            if msg.is_none() {
                msg = Some(tr("Ajustes guardados: se aplicarán al abrir la cámara"));
            }
        }
        if let Some(m) = msg {
            self.say(m);
        }
    }
}

/// Resolution, frames per second and compression, like the original's camera format picker.
fn format_picker(ui: &mut egui::Ui, formats: &[v4l::FormatInfo], cur: Mode) -> Option<Mode> {
    let mut out = None;
    let Some(f) = formats.iter().find(|f| f.fourcc == cur.0).or(formats.first()) else {
        ui.label(RichText::new(tr("La cámara no ofrece formatos de vídeo.")).size(12.0).color(TEXT_DIM));
        return None;
    };
    let w = (ui.available_width() * 0.55).max(120.0);
    let row = |ui: &mut egui::Ui, label: &str, add: &mut dyn FnMut(&mut egui::Ui)| {
        split_row(
            ui,
            |ui| {
                ui.add(egui::Label::new(RichText::new(tr(label)).size(12.5).color(TEXT_2)).truncate());
            },
            |ui| add(ui),
        );
    };
    row(ui, "Resolución", &mut |ui| {
        egui::ComboBox::from_id_salt("fmt_res").selected_text(mode_label((cur.0, cur.1, cur.2, 0, 0))).width(w).show_ui(ui, |ui| {
            for s in &f.sizes {
                if ui.selectable_label((cur.1, cur.2) == *s, format!("{} ({}×{})", mode_label((0, s.0, s.1, 0, 0)), s.0, s.1)).clicked() {
                    let ivs = f.fps_for(s.0, s.1);
                    let iv = ivs.iter().find(|iv| **iv == (cur.3, cur.4)).or(ivs.first()).copied().unwrap_or((0, 0));
                    out = Some((f.fourcc, s.0, s.1, iv.0, iv.1));
                }
            }
        });
    });
    row(ui, "Fotogramas/seg.", &mut |ui| {
        let fps = v4l::fps_value((cur.3, cur.4));
        egui::ComboBox::from_id_salt("fmt_fps").selected_text(format!("{fps:.0} fps")).width(w).show_ui(ui, |ui| {
            for iv in f.fps_for(cur.1, cur.2) {
                if ui.selectable_label((cur.3, cur.4) == iv, format!("{:.0} fps", v4l::fps_value(iv))).clicked() {
                    out = Some((cur.0, cur.1, cur.2, iv.0, iv.1));
                }
            }
        });
    });
    row(ui, "Compresión", &mut |ui| {
        egui::ComboBox::from_id_salt("fmt_cc").selected_text(fmt_name(cur.0)).width(w).show_ui(ui, |ui| {
            for g in formats {
                if ui.selectable_label(g.fourcc == cur.0, fmt_name(g.fourcc)).clicked() && g.fourcc != cur.0 {
                    let s = g.sizes.iter().find(|s| (s.0, s.1) == (cur.1, cur.2)).or(g.sizes.first()).copied().unwrap_or((0, 0));
                    let ivs = g.fps_for(s.0, s.1);
                    let iv = ivs.iter().find(|iv| **iv == (cur.3, cur.4)).or(ivs.first()).copied().unwrap_or((0, 0));
                    out = Some((g.fourcc, s.0, s.1, iv.0, iv.1));
                }
            }
        });
    });
    out
}

fn status_chip(ui: &mut egui::Ui, s: &camera::Status) {
    let (text, col) = match s {
        camera::Status::Driver => ("Linux", SUCCESS),
        camera::Status::Usb => ("USB directo", ACCENT),
        camera::Status::App => ("En Lente", SUCCESS),
        camera::Status::NeedsPermission => ("Falta permiso", WARNING),
        camera::Status::NotOnCamera => ("La cámara no lo tiene", TEXT_DIM),
        camera::Status::WindowsOnly(_) => ("Sólo Windows", Color32::from_rgb(0xe8, 0x6a, 0x5a)),
        camera::Status::Pending => ("Al conectar", TEXT_DIM),
    };
    let g = ui.painter().layout_no_wrap(tr(text), FontId::proportional(11.0), col);
    let (r, _) = ui.allocate_exact_size(g.size() + Vec2::new(12.0, 6.0), Sense::hover());
    ui.painter().rect_filled(r, CornerRadius::same(4), col.gamma_multiply(0.18));
    ui.painter().galley(r.min + Vec2::new(6.0, 3.0), g, col);
}

fn launcher_command() -> String {
    let exe = std::env::current_exe().ok();
    let script = exe
        .as_ref()
        .and_then(|e| e.parent()?.parent()?.parent().map(|d| d.join("abrir-lente.sh")))
        .filter(|p| p.exists());
    match (script, exe) {
        (Some(s), _) => format!("sh \"{}\"", s.display()),
        (None, Some(e)) => format!("\"{}\"", e.display()),
        _ => "lente".into(),
    }
}

fn fmt_name(f: u32) -> String {
    match f {
        v4l::FMT_MJPG => "MJPEG".into(),
        v4l::FMT_YUYV => "YUY2".into(),
        v4l::FMT_NV12 => "NV12".into(),
        other => v4l::fourcc_str(other),
    }
}

fn short(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        format!("{}…", s.chars().take(n - 1).collect::<String>())
    }
}

// ------------------------------------------------------------------ widgets

fn card(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(CARD)
        .stroke(Stroke::new(1.0, Color32::from_white_alpha(15)))
        .corner_radius(CornerRadius::same(4))
        .inner_margin(Margin::symmetric(14, 12))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui)
        });
}

fn expander_header(ui: &mut egui::Ui, title: &str, open: bool) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 46.0), Sense::click());
    let p = ui.painter();
    let fill = if resp.hovered() { Color32::from_rgb(0x32, 0x32, 0x32) } else { CARD };
    let radius = if open { CornerRadius { nw: 4, ne: 4, sw: 0, se: 0 } } else { CornerRadius::same(4) };
    p.rect_filled(rect, radius, fill);
    p.rect_stroke(rect, radius, Stroke::new(1.0, Color32::from_white_alpha(15)), StrokeKind::Inside);
    p.text(pos2(rect.left() + 16.0, rect.center().y), Align2::LEFT_CENTER, tr(title), FontId::proportional(14.0), TEXT);
    let c = pos2(rect.right() - 20.0, rect.center().y);
    let pts = if open {
        vec![c + Vec2::new(-5.0, 2.5), c + Vec2::new(0.0, -2.5), c + Vec2::new(5.0, 2.5)]
    } else {
        vec![c + Vec2::new(-5.0, -2.5), c + Vec2::new(0.0, 2.5), c + Vec2::new(5.0, -2.5)]
    };
    p.add(egui::Shape::line(pts, Stroke::new(1.3, TEXT_2)));
    resp
}

fn tab_button(ui: &mut egui::Ui, label: &str, selected: bool) -> egui::Response {
    let font = if selected { semibold(14.0) } else { FontId::proportional(14.0) };
    let galley = ui.painter().layout_no_wrap(tr(label), font, if selected { TEXT } else { TEXT_DIM });
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(galley.size().x + 20.0, 34.0), Sense::click());
    let p = ui.painter();
    if resp.hovered() && !selected {
        p.rect_filled(rect, CornerRadius::same(4), hover_bg());
    }
    p.galley(pos2(rect.center().x - galley.size().x / 2.0, rect.center().y - galley.size().y / 2.0 - 2.0), galley, TEXT);
    if selected {
        let bar = Rect::from_center_size(pos2(rect.center().x, rect.bottom() - 2.0), Vec2::new(16.0, 3.0));
        p.rect_filled(bar, CornerRadius::same(2), ACCENT);
    }
    resp
}

fn accent_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    let galley = ui.painter().layout_no_wrap(label.to_string(), FontId::proportional(14.0), Color32::BLACK);
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(galley.size().x + 24.0, 32.0), Sense::click());
    let fill = if resp.is_pointer_button_down_on() { ACCENT_HOVER.gamma_multiply(0.9) } else if resp.hovered() { ACCENT_HOVER } else { ACCENT };
    ui.painter().rect_filled(rect, CornerRadius::same(4), fill);
    ui.painter().galley(rect.center() - galley.size() / 2.0, galley, Color32::BLACK);
    resp
}

fn preset_chip(ui: &mut egui::Ui, n: usize, has: bool) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(68.0, 30.0), Sense::click());
    let p = ui.painter();
    let fill = if resp.hovered() { Color32::from_rgb(0x38, 0x38, 0x38) } else { ELEVATED };
    p.rect_filled(rect, CornerRadius::same(4), fill);
    let stroke = if has { Color32::from_rgba_unmultiplied(0x60, 0xcd, 0xff, 110) } else { border() };
    p.rect_stroke(rect, CornerRadius::same(4), Stroke::new(1.0, stroke), StrokeKind::Inside);
    let t = if has { format!("{n}") } else { format!("+ {n}") };
    p.text(rect.center(), Align2::CENTER_CENTER, t, FontId::proportional(13.0), if has { ACCENT } else { TEXT_2 });
    resp
}

fn control_row(ui: &mut egui::Ui, c: &camera::Control) -> Option<i32> {
    let enabled = !c.inactive && !c.read_only;
    let mut out = None;
    ui.add_enabled_ui(enabled, |ui| match &c.kind {
        Kind::Toggle => {
            split_row(
                ui,
                |ui| label_with_via(ui, c),
                |ui| {
                    ui.set_min_height(32.0);
                    let mut on = c.value != 0;
                    if toggle(ui, &mut on).clicked() {
                        out = Some(on as i32);
                    }
                },
            );
        }
        Kind::Menu(items) => {
            label_with_via(ui, c);
            let cur = items.iter().find(|(v, _)| *v == c.value).map(|(_, s)| tr(s)).unwrap_or(c.value.to_string());
            egui::ComboBox::from_id_salt(&c.key).selected_text(cur).truncate().width(ui.available_width() - 2.0).show_ui(ui, |ui| {
                for (v, s) in items {
                    if ui.selectable_label(*v == c.value, tr(s)).clicked() && *v != c.value {
                        out = Some(*v);
                    }
                }
            });
            ui.add_space(4.0);
        }
        Kind::Button => {
            if ui.button(tr(&c.label)).clicked() {
                out = Some(1);
            }
        }
        Kind::Slider => {
            split_row(
                ui,
                |ui| label_with_via(ui, c),
                |ui| {
                    ui.label(RichText::new(value_text(c)).size(13.0).color(TEXT_2));
                    if c.value != c.default {
                        let r = icon_button_sized(ui, Icon::Refresh, &tr_n("Volver a {}", c.default), 22.0);
                        if r.clicked() {
                            out = Some(c.default);
                        }
                    }
                },
            );
            if let Some(v) = win_slider(ui, c.value, c.min, c.max, c.step.max(1)) {
                out = Some(v);
            }
            ui.add_space(4.0);
        }
    });
    if c.inactive {
        ui.label(RichText::new(tr("Controlado automáticamente")).size(11.0).color(TEXT_FAINT));
    }
    out
}

/// WinUI-style slider: thin track, accent fill, round thumb with accent core.
fn win_slider(ui: &mut egui::Ui, value: i32, min: i32, max: i32, step: i32) -> Option<i32> {
    let w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, 24.0), Sense::click_and_drag());
    let (lo, hi) = (min.min(max), max.max(min));
    let span = (hi - lo).max(1) as f32;
    let track = Rect::from_min_max(pos2(rect.left() + 10.0, rect.center().y - 2.0), pos2(rect.right() - 10.0, rect.center().y + 2.0));
    let mut out = None;
    if let Some(pos) = resp.interact_pointer_pos() {
        if resp.dragged() || resp.clicked() {
            let t = ((pos.x - track.left()) / track.width()).clamp(0.0, 1.0);
            let raw = lo as f32 + t * span;
            let stepped = lo + (((raw - lo as f32) / step as f32).round() as i32) * step;
            let v = stepped.clamp(lo, hi);
            if v != value {
                out = Some(v);
            }
        }
    }
    let shown = out.unwrap_or(value);
    let t = ((shown - lo) as f32 / span).clamp(0.0, 1.0);
    let x = egui::lerp(track.left()..=track.right(), t);
    let p = ui.painter();
    let enabled = ui.is_enabled();
    let acc = if enabled { ACCENT } else { Color32::from_gray(90) };
    p.rect_filled(track, CornerRadius::same(2), Color32::from_white_alpha(if enabled { 90 } else { 40 }));
    p.rect_filled(Rect::from_min_max(track.min, pos2(x, track.max.y)), CornerRadius::same(2), acc);
    p.circle_filled(pos2(x, rect.center().y), 10.0, Color32::from_rgb(0x45, 0x45, 0x45));
    p.circle_stroke(pos2(x, rect.center().y), 10.0, Stroke::new(1.0, Color32::from_white_alpha(20)));
    let core = if resp.is_pointer_button_down_on() { 5.0 } else if resp.hovered() { 7.0 } else { 6.0 };
    p.circle_filled(pos2(x, rect.center().y), core, acc);
    out
}

/// WinUI toggle switch.
fn toggle(ui: &mut egui::Ui, on: &mut bool) -> egui::Response {
    let (rect, mut resp) = ui.allocate_exact_size(Vec2::new(40.0, 20.0), Sense::click());
    if resp.clicked() {
        *on = !*on;
        resp.mark_changed();
    }
    let t = ui.ctx().animate_bool_responsive(resp.id, *on);
    let p = ui.painter();
    let enabled = ui.is_enabled();
    if *on {
        p.rect_filled(rect, CornerRadius::same(10), if enabled { ACCENT } else { Color32::from_gray(80) });
    } else {
        p.rect_stroke(rect, CornerRadius::same(10), Stroke::new(1.0, Color32::from_white_alpha(150)), StrokeKind::Inside);
    }
    let x = egui::lerp(rect.left() + 10.0..=rect.right() - 10.0, t);
    let knob = if *on { Color32::BLACK } else { Color32::from_white_alpha(200) };
    let r = if resp.hovered() { 6.0 } else { 5.0 };
    p.circle_filled(pos2(x, rect.center().y), r, knob);
    resp
}

fn badge(p: &egui::Painter, at: Pos2, text: &str, col: Color32) {
    let g = p.layout_no_wrap(text.to_string(), FontId::proportional(12.0), Color32::BLACK);
    let r = Rect::from_min_size(at, g.size() + Vec2::new(16.0, 8.0));
    p.rect_filled(r, CornerRadius::same(4), col);
    p.galley(at + Vec2::new(8.0, 4.0), g, Color32::BLACK);
}

#[derive(Clone, Copy)]
enum Icon {
    Fullscreen,
    Grid,
    Refresh,
    Play,
    Pause,
    Camera,
    Folder,
    Pane,
    Hud,
    Mirror,
}

fn icon_button(ui: &mut egui::Ui, icon: Icon, tip: &str) -> egui::Response {
    icon_button_sized(ui, icon, tip, 30.0)
}

fn icon_button_sized(ui: &mut egui::Ui, icon: Icon, tip: &str, s: f32) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(s), Sense::click());
    if resp.hovered() {
        ui.painter().rect_filled(rect, CornerRadius::same(4), hover_bg());
    }
    draw_icon(ui.painter(), rect, icon, TEXT);
    resp.on_hover_text(tip)
}

fn icon_button_toggle(ui: &mut egui::Ui, icon: Icon, tip: &str, on: bool) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(36.0), Sense::click());
    let p = ui.painter();
    if on {
        p.rect_filled(rect, CornerRadius::same(4), Color32::from_rgba_unmultiplied(0x60, 0xcd, 0xff, 30));
    } else if resp.hovered() {
        p.rect_filled(rect, CornerRadius::same(4), hover_bg());
    }
    draw_icon(p, rect, icon, if on { ACCENT } else { TEXT });
    resp.on_hover_text(tip)
}

fn draw_icon(p: &egui::Painter, rect: Rect, icon: Icon, col: Color32) {
    let c = rect.center();
    let s = Stroke::new(1.5, col);
    match icon {
        Icon::Refresh => {
            let pts: Vec<Pos2> = (0..=20)
                .map(|i| {
                    let a = 0.6 + i as f32 / 20.0 * 5.0;
                    c + Vec2::new(a.cos(), a.sin()) * 6.5
                })
                .collect();
            let end = *pts.last().unwrap();
            p.add(egui::Shape::line(pts, s));
            p.line_segment([end, end + Vec2::new(3.5, -0.5)], s);
            p.line_segment([end, end + Vec2::new(0.5, -3.5)], s);
        }
        Icon::Play => {
            p.add(egui::Shape::convex_polygon(vec![c + Vec2::new(-4.0, -6.0), c + Vec2::new(6.0, 0.0), c + Vec2::new(-4.0, 6.0)], col, Stroke::NONE));
        }
        Icon::Pause => {
            p.rect_filled(Rect::from_center_size(c + Vec2::new(-3.5, 0.0), Vec2::new(3.0, 12.0)), CornerRadius::same(1), col);
            p.rect_filled(Rect::from_center_size(c + Vec2::new(3.5, 0.0), Vec2::new(3.0, 12.0)), CornerRadius::same(1), col);
        }
        Icon::Camera => {
            let body = Rect::from_center_size(c + Vec2::new(0.0, 1.0), Vec2::new(16.0, 11.0));
            p.rect_stroke(body, CornerRadius::same(2), s, StrokeKind::Middle);
            p.circle_stroke(body.center(), 3.0, s);
            p.line_segment([pos2(body.left() + 4.0, body.top()), pos2(body.left() + 6.0, body.top() - 2.5)], s);
            p.line_segment([pos2(body.left() + 6.0, body.top() - 2.5), pos2(body.left() + 10.0, body.top() - 2.5)], s);
            p.line_segment([pos2(body.left() + 10.0, body.top() - 2.5), pos2(body.left() + 12.0, body.top())], s);
        }
        Icon::Folder => {
            let r = Rect::from_center_size(c + Vec2::new(0.0, 1.0), Vec2::new(16.0, 11.0));
            p.rect_stroke(r, CornerRadius::same(2), s, StrokeKind::Middle);
            p.line_segment([pos2(r.left(), r.top()), pos2(r.left() + 2.0, r.top() - 2.5)], s);
            p.line_segment([pos2(r.left() + 2.0, r.top() - 2.5), pos2(r.left() + 6.0, r.top() - 2.5)], s);
            p.line_segment([pos2(r.left() + 6.0, r.top() - 2.5), pos2(r.left() + 7.5, r.top())], s);
        }
        Icon::Pane => {
            let r = Rect::from_center_size(c, Vec2::new(16.0, 13.0));
            p.rect_stroke(r, CornerRadius::same(2), s, StrokeKind::Middle);
            p.line_segment([pos2(r.right() - 5.5, r.top()), pos2(r.right() - 5.5, r.bottom())], s);
        }
        Icon::Hud => {
            let r = Rect::from_center_size(c, Vec2::new(16.0, 12.0));
            p.rect_stroke(r, CornerRadius::same(2), s, StrokeKind::Middle);
            p.rect_filled(Rect::from_min_max(pos2(r.left() + 3.0, r.bottom() - 4.5), pos2(r.right() - 3.0, r.bottom() - 2.0)), CornerRadius::same(1), col);
        }
        Icon::Fullscreen => {
            let r = Rect::from_center_size(c, Vec2::new(14.0, 12.0));
            for (a, dx, dy) in [(r.left_top(), 1.0, 1.0), (r.right_top(), -1.0, 1.0), (r.left_bottom(), 1.0, -1.0), (r.right_bottom(), -1.0, -1.0)] {
                p.line_segment([a, a + Vec2::new(4.0 * dx, 0.0)], s);
                p.line_segment([a, a + Vec2::new(0.0, 4.0 * dy)], s);
            }
        }
        Icon::Grid => {
            let r = Rect::from_center_size(c, Vec2::new(15.0, 12.0));
            p.rect_stroke(r, CornerRadius::same(1), Stroke::new(1.2, col), StrokeKind::Middle);
            for k in 1..3 {
                let x = r.left() + r.width() * k as f32 / 3.0;
                let y = r.top() + r.height() * k as f32 / 3.0;
                p.line_segment([pos2(x, r.top()), pos2(x, r.bottom())], Stroke::new(1.0, col));
                p.line_segment([pos2(r.left(), y), pos2(r.right(), y)], Stroke::new(1.0, col));
            }
        }
        Icon::Mirror => {
            p.line_segment([c + Vec2::new(0.0, -7.0), c + Vec2::new(0.0, 7.0)], Stroke::new(1.0, col));
            p.add(egui::Shape::closed_line(vec![c + Vec2::new(-2.5, -5.0), c + Vec2::new(-2.5, 5.0), c + Vec2::new(-8.0, 5.0)], s));
            p.add(egui::Shape::convex_polygon(vec![c + Vec2::new(2.5, -5.0), c + Vec2::new(8.0, 5.0), c + Vec2::new(2.5, 5.0)], col, Stroke::NONE));
        }
    }
}

fn app_logo(ui: &mut egui::Ui) {
    let (r, _) = ui.allocate_exact_size(Vec2::new(22.0, 22.0), Sense::hover());
    let p = ui.painter();
    p.rect_filled(r, CornerRadius::same(5), ACCENT);
    p.circle_stroke(r.center(), 5.5, Stroke::new(2.0, Color32::BLACK));
    p.circle_filled(r.center(), 2.0, Color32::BLACK);
}

/// Simple drawn webcam (placeholder for the original's product photos).
fn device_glyph(p: &egui::Painter, r: Rect, elgato: bool) {
    let c = r.center();
    let w = r.width();
    let body = Rect::from_center_size(c - Vec2::new(0.0, w * 0.08), Vec2::new(w * 0.86, w * 0.42));
    p.rect_filled(body, CornerRadius::same((w * 0.12) as u8), Color32::from_rgb(0x3a, 0x3a, 0x3a));
    p.rect_filled(body.shrink(1.5), CornerRadius::same((w * 0.1) as u8), Color32::from_rgb(0x15, 0x15, 0x15));
    p.circle_filled(body.center(), w * 0.14, Color32::from_rgb(0x2c, 0x2c, 0x2c));
    p.circle_filled(body.center(), w * 0.09, if elgato { Color32::from_rgb(0x1f, 0x4e, 0x66) } else { Color32::from_gray(70) });
    p.circle_filled(body.center() + Vec2::new(-w * 0.03, -w * 0.03), w * 0.025, Color32::from_white_alpha(120));
    let stand = Rect::from_center_size(pos2(c.x, body.bottom() + w * 0.12), Vec2::new(w * 0.08, w * 0.2));
    p.rect_filled(stand, CornerRadius::ZERO, Color32::from_rgb(0x30, 0x30, 0x30));
    let foot = Rect::from_center_size(pos2(c.x, stand.bottom()), Vec2::new(w * 0.4, w * 0.06));
    p.rect_filled(foot, CornerRadius::same(2), Color32::from_rgb(0x30, 0x30, 0x30));
}

fn big_camera_glyph(ui: &mut egui::Ui) {
    let (r, _) = ui.allocate_exact_size(Vec2::new(64.0, 48.0), Sense::hover());
    let p = ui.painter();
    let s = Stroke::new(2.0, TEXT_DIM);
    let body = Rect::from_center_size(r.center() + Vec2::new(-6.0, 0.0), Vec2::new(40.0, 30.0));
    p.rect_stroke(body, CornerRadius::same(5), s, StrokeKind::Middle);
    p.add(egui::Shape::closed_line(
        vec![
            pos2(body.right() + 3.0, r.center().y - 4.0),
            pos2(body.right() + 15.0, r.center().y - 11.0),
            pos2(body.right() + 15.0, r.center().y + 11.0),
            pos2(body.right() + 3.0, r.center().y + 4.0),
        ],
        s,
    ));
}

/// A row with a control on the right that keeps its size and a label on the left that is cut
/// with "…" when it doesn't fit (no text ever runs under the control or out of the panel).
fn split_row(ui: &mut egui::Ui, left: impl FnOnce(&mut egui::Ui), right: impl FnOnce(&mut egui::Ui)) {
    ui.horizontal(|ui| {
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            right(ui);
            ui.with_layout(Layout::left_to_right(Align::Center), left);
        });
    });
}

fn label_with_via(ui: &mut egui::Ui, c: &camera::Control) {
    ui.add(egui::Label::new(tr(&c.label)).truncate());
    if c.via == Some(camera::Via::Usb) {
        ui.label(RichText::new(tr("USB")).size(10.0).color(ACCENT)).on_hover_text(tr("Linux no expone este ajuste; Lente lo envía directo a la cámara por USB"));
    }
}

/// How a value is shown: exposure time (100 µs units) as a shutter speed, the rest as is.
fn value_text(c: &camera::Control) -> String {
    match c.source {
        camera::Source::Std(elgato::CID_EXPOSURE_ABS) if c.value > 0 => {
            let denom = (10_000.0 / c.value as f32).round();
            if denom >= 1.0 {
                format!("1/{denom:.0} s")
            } else {
                format!("{:.1} s", c.value as f32 / 10_000.0)
            }
        }
        _ => c.value.to_string(),
    }
}
