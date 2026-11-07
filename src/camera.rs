//! Camera discovery, the control model shown in the settings panel, and the per-feature
//! support report.
//!
//! Elgato controls go through Linux's `uvcvideo` driver when the camera lists them; otherwise
//! they are sent straight to the camera over USB (see usb.rs).

use crate::elgato::{self, Group, XuDef};
use crate::i18n::{tr, tr_n, trf};
use crate::usb::DirectUsb;
use crate::v4l::{self, Device};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq)]
pub struct XuUnit {
    pub id: u8,
    pub guid: String,
    /// Selectors the camera lists in bmControls (the only ones uvcvideo lets through).
    pub selectors: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct CameraEntry {
    pub path: String,
    pub name: String,
    pub vendor: u16,
    pub product: u16,
    pub serial: String,
    pub elgato_webcam: bool,
    pub xu_units: Vec<XuUnit>,
    /// /dev/bus/usb/BBB/DDD
    pub usb_node: Option<String>,
    pub vc_interface: u8,
    pub error: Option<String>,
    pub demo: bool,
}

impl CameraEntry {
    pub fn profile_id(&self) -> String {
        let s = if self.serial.is_empty() { self.name.replace(' ', "_") } else { self.serial.clone() };
        format!("{:04x}-{:04x}-{}", self.vendor, self.product, s)
    }
    pub fn advertised(&self, unit: u8, selector: u8) -> bool {
        self.xu_units.iter().any(|u| u.id == unit && u.selectors.contains(&selector))
    }
}

fn read_trim(p: &PathBuf) -> Option<String> {
    std::fs::read_to_string(p).ok().map(|s| s.trim().to_string())
}

/// Extension units (unit id, GUID, listed selectors) from the USB configuration descriptors.
fn parse_xu_units(d: &[u8]) -> Vec<XuUnit> {
    let mut out = Vec::new();
    let mut i = 0;
    while i + 2 <= d.len() {
        let len = d[i] as usize;
        if len < 2 || i + len > d.len() {
            break;
        }
        let b = &d[i..i + len];
        if b[1] == 0x24 && len >= 24 && b[2] == 0x06 {
            let g = &b[4..20];
            let guid = format!(
                "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
                g[3], g[2], g[1], g[0], g[5], g[4], g[7], g[6], g[8], g[9], g[10], g[11], g[12], g[13], g[14], g[15]
            );
            let pins = b[21] as usize;
            let mut selectors = Vec::new();
            if 22 + pins < len {
                let size = b[22 + pins] as usize;
                let start = 23 + pins;
                for byte in 0..size {
                    if start + byte >= len {
                        break;
                    }
                    for bit in 0..8 {
                        if b[start + byte] & (1 << bit) != 0 {
                            selectors.push((byte * 8 + bit + 1) as u8);
                        }
                    }
                }
            }
            out.push(XuUnit { id: b[3], guid, selectors });
        }
        i += len;
    }
    out
}

pub fn discover() -> Vec<CameraEntry> {
    let mut out = Vec::new();
    let Ok(rd) = std::fs::read_dir("/sys/class/video4linux") else { return out };
    let mut nodes: Vec<_> = rd.flatten().map(|e| e.path()).collect();
    nodes.sort_by_key(|p| {
        p.file_name()
            .and_then(|n| n.to_str())
            .and_then(|n| n.trim_start_matches("video").parse::<u32>().ok())
            .unwrap_or(999)
    });
    for node in nodes {
        let fname = node.file_name().unwrap().to_string_lossy().to_string();
        if !fname.starts_with("video") {
            continue;
        }
        let dev_path = format!("/dev/{fname}");
        let sys_name = read_trim(&node.join("name")).unwrap_or_else(|| fname.clone());
        let index = read_trim(&node.join("index")).and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
        let intf = node.join("device");
        let usb = intf.join("..");
        let vendor = read_trim(&usb.join("idVendor")).and_then(|s| u16::from_str_radix(&s, 16).ok()).unwrap_or(0);
        let product = read_trim(&usb.join("idProduct")).and_then(|s| u16::from_str_radix(&s, 16).ok()).unwrap_or(0);
        let serial = read_trim(&usb.join("serial")).unwrap_or_default();
        let xu_units = std::fs::read(usb.join("descriptors")).map(|d| parse_xu_units(&d)).unwrap_or_default();
        let usb_node = match (read_trim(&usb.join("busnum")), read_trim(&usb.join("devnum"))) {
            (Some(b), Some(d)) => b.parse::<u32>().ok().zip(d.parse::<u32>().ok()).map(|(b, d)| format!("/dev/bus/usb/{b:03}/{d:03}")),
            _ => None,
        };
        let vc_interface = read_trim(&intf.join("bInterfaceNumber")).and_then(|s| u8::from_str_radix(&s, 16).ok()).unwrap_or(0);
        let (model, elgato_webcam) = match elgato::model(vendor, product) {
            Some((m, w)) => (Some(m), w),
            None => (None, false),
        };
        let mut name = model.map(|m| format!("Elgato {m}")).unwrap_or(sys_name.clone());
        let error = match Device::open(&dev_path) {
            Ok(dev) => match dev.query_cap() {
                Ok(cap) => {
                    let caps = if cap.capabilities & v4l::CAP_DEVICE_CAPS != 0 { cap.device_caps } else { cap.capabilities };
                    if caps & v4l::CAP_VIDEO_CAPTURE == 0 {
                        continue; // metadata node
                    }
                    if model.is_none() {
                        let card = v4l::cstr(&cap.card);
                        if !card.is_empty() {
                            name = card;
                        }
                    }
                    None
                }
                Err(e) => Some(format!("No se pudo leer: {e}")),
            },
            Err(e) => {
                if index != 0 {
                    continue;
                }
                Some(match e.raw_os_error() {
                    Some(libc::EACCES) | Some(libc::EPERM) => {
                        "Permiso denegado: añade tu usuario al grupo «video» y vuelve a iniciar sesión.".to_string()
                    }
                    Some(libc::ENOENT) => {
                        "La cámara existe pero /dev no es visible (¿app en sandbox?). Ejecuta la app fuera del sandbox.".to_string()
                    }
                    Some(libc::EBUSY) => "La cámara está ocupada por otra aplicación.".to_string(),
                    _ => format!("No se pudo abrir: {e}"),
                })
            }
        };
        out.push(CameraEntry {
            path: dev_path,
            name,
            vendor,
            product,
            serial,
            elgato_webcam,
            xu_units,
            usb_node,
            vc_interface,
            error,
            demo: false,
        });
    }
    out
}

pub fn demo_entry() -> CameraEntry {
    CameraEntry {
        path: "demo".into(),
        name: "Elgato Facecam 4K (demostración)".into(),
        vendor: elgato::ELGATO_VENDOR,
        product: 0x92,
        serial: "DEMO".into(),
        elgato_webcam: true,
        xu_units: vec![],
        usb_node: None,
        vc_interface: 0,
        error: None,
        demo: true,
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Kind {
    Slider,
    Toggle,
    Menu(Vec<(i32, String)>),
    Button,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Via {
    Driver,
    Usb,
}

#[derive(Clone, Debug)]
pub enum Source {
    Std(u32),
    Xu(XuDef),
    /// Rango dinámico: escribe HDR (XU1 21) y compensación de contraluz.
    DynamicRange { hdr: XuDef, blc: Option<u32> },
    /// Compensación de exposición: escribe Brillo, sólo con exposición automática (0x1802ccd70).
    EvComp(u32),
    Demo,
}

#[derive(Clone, Debug)]
pub struct Control {
    pub key: String,
    pub label: String,
    pub group: Group,
    pub order: i32,
    pub kind: Kind,
    pub min: i32,
    pub max: i32,
    pub step: i32,
    pub default: i32,
    pub value: i32,
    pub source: Source,
    pub inactive: bool,
    pub read_only: bool,
    pub via: Option<Via>,
}

/// One feature of the original app and how it works here.
#[derive(Clone, Debug, PartialEq)]
pub enum Status {
    /// Works through the Linux driver.
    Driver,
    /// Works by talking to the camera directly over USB.
    Usb,
    /// The camera answers it directly over USB, once the USB permission is installed.
    NeedsPermission,
    /// This camera doesn't have it.
    NotOnCamera,
    /// Only possible in Elgato's Windows app, with the reason.
    WindowsOnly(&'static str),
    /// Not checked yet (camera not reachable).
    Pending,
    /// In Lente itself, no camera command needed.
    App,
}

#[derive(Clone, Debug)]
pub struct Feature {
    pub name: &'static str,
    pub section: &'static str,
    pub status: Status,
    pub note: String,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Command {
    Refocus,
    AutoWhiteBalanceOnce,
    AutoExposureOnce,
    SaveToCamera,
    FactoryReset,
    Reboot,
}

pub use elgato::CameraState;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Led {
    /// 0 apagada, 1 al transmitir, 2 siempre
    pub mode: i32,
    pub percent: i32,
}

/// An opened camera: device handle plus its controls.
pub struct Camera {
    pub entry: CameraEntry,
    pub dev: Option<Device>,
    pub usb: Option<DirectUsb>,
    pub usb_error: Option<String>,
    pub controls: Vec<Control>,
    pub formats: Vec<v4l::FormatInfo>,
    pub skipped_xu: Vec<String>,
    pub readouts: Vec<(String, Option<i64>)>,
    /// What the camera reports it is sending (XU1 CAMERA_STATUS), see camera_state().
    pub sensor: Option<CameraState>,
    pub features: Vec<Feature>,
    pub led: Option<Led>,
    pub led_via: Option<Via>,
    pub commands: Vec<(Command, Option<Via>)>,
    /// Settings stored inside the camera (what Windows saved), see read_camera_profile.
    pub camera_profile: Option<Vec<ProfileRow>>,
    /// Throttled writes (as the original's Throttler): control index → value not sent yet.
    pending: BTreeMap<usize, i32>,
    last_write: BTreeMap<usize, std::time::Instant>,
}

#[derive(Debug)]
pub enum XuError {
    /// Neither path works because USB access is missing.
    NoPermission,
    Failed(String),
}

impl std::fmt::Display for XuError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            XuError::NoPermission => write!(f, "{}", tr("falta el permiso USB")),
            XuError::Failed(e) => write!(f, "{e}"),
        }
    }
}

impl Camera {
    pub fn open(entry: &CameraEntry) -> Result<Camera, String> {
        if entry.demo || entry.error.is_some() {
            // Sin acceso: se muestran los ajustes típicos de la cámara, sin conexión.
            return Ok(Self::demo(entry.clone()));
        }
        let dev = Device::open(&entry.path).map_err(|e| e.to_string())?;
        let (usb, usb_error) = match (&entry.usb_node, entry.elgato_webcam) {
            (Some(node), true) => match DirectUsb::open(node, entry.vc_interface) {
                Ok(u) => (Some(u), None),
                Err(e) => (None, Some(e.to_string())),
            },
            _ => (None, None),
        };
        let mut cam = Camera {
            entry: entry.clone(),
            dev: Some(dev),
            usb,
            usb_error,
            controls: vec![],
            formats: vec![],
            skipped_xu: vec![],
            readouts: vec![],
            sensor: None,
            features: vec![],
            led: None,
            led_via: None,
            commands: vec![],
            camera_profile: None,
            pending: BTreeMap::new(),
            last_write: BTreeMap::new(),
        };
        let dev = cam.dev.as_ref().unwrap();
        let mut controls = Vec::new();
        for c in dev.controls() {
            let (label, group, order) = elgato::std_control_meta(c.id, &c.name);
            let kind = match c.ty {
                v4l::CTRL_TYPE_BOOLEAN => Kind::Toggle,
                v4l::CTRL_TYPE_MENU | v4l::CTRL_TYPE_INTEGER_MENU => {
                    Kind::Menu(c.menu.iter().map(|(i, s)| (*i, elgato::translate_menu(s))).collect())
                }
                v4l::CTRL_TYPE_BUTTON => Kind::Button,
                _ => Kind::Slider,
            };
            let value = dev.get_ctrl(c.id).unwrap_or(c.default);
            controls.push(Control {
                key: format!("std_{:08x}", c.id),
                label,
                group,
                order,
                kind,
                min: c.min,
                max: c.max,
                step: c.step.max(1),
                default: c.default,
                value,
                source: Source::Std(c.id),
                inactive: c.flags & v4l::CTRL_FLAG_INACTIVE != 0,
                read_only: c.flags & v4l::CTRL_FLAG_READ_ONLY != 0,
                via: Some(Via::Driver),
            });
        }
        cam.formats = dev.formats();
        let mut xu_status: BTreeMap<&'static str, Result<Via, bool>> = BTreeMap::new();
        if entry.elgato_webcam {
            let blc = elgato::original_prop_to_v4l2(elgato::PROP_BACKLIGHT_COMPENSATION)
                .filter(|id| controls.iter().any(|c| matches!(c.source, Source::Std(i) if i == *id)));
            for def in elgato::XU_CONTROLS {
                let mut buf = vec![0u8; def.size as usize];
                match cam.xu(def.unit, def.selector, v4l::UVC_GET_CUR, &mut buf) {
                    Ok(via) => {
                        xu_status.insert(def.key, Ok(via));
                        let value = elgato::decode(def, &buf);
                        if def.key == "xu_hdr" {
                            let blc_v = blc.and_then(|id| cam.dev.as_ref().unwrap().get_ctrl(id).ok()).unwrap_or(0);
                            let mut menu = vec![(0, "Estándar".to_string())];
                            if blc.is_some() {
                                menu.push((1, "Amplio".to_string()));
                            }
                            menu.push((2, "Alto (HDR)".to_string()));
                            let v = elgato::dynamic_range_read(value, blc_v, blc.is_some());
                            controls.push(Control {
                                key: "dynamic_range".into(),
                                label: "Rango dinámico".into(),
                                group: Group::Picture,
                                order: 10,
                                kind: Kind::Menu(menu),
                                min: 0,
                                max: 2,
                                step: 1,
                                default: 0,
                                value: v,
                                source: Source::DynamicRange { hdr: *def, blc },
                                inactive: false,
                                read_only: false,
                                via: Some(via),
                            });
                            continue;
                        }
                        let mut c = xu_control(def, xu_display_value(def, value));
                        c.via = Some(via);
                        controls.push(c);
                    }
                    Err(e) => {
                        xu_status.insert(def.key, Err(matches!(e, XuError::NoPermission)));
                        cam.skipped_xu.push(def.label.to_string());
                    }
                }
            }
            // Compensación de exposición (propiedad virtual del original): actúa sobre Brillo.
            if let Some(bid) = elgato::original_prop_to_v4l2(elgato::PROP_BRIGHTNESS) {
                if let Some(b) = controls.iter().find(|c| matches!(c.source, Source::Std(i) if i == bid)).cloned() {
                    let ae_auto = controls
                        .iter()
                        .find(|c| matches!(c.source, Source::Std(elgato::CID_EXPOSURE_AUTO)))
                        .map(|c| c.value != 1)
                        .unwrap_or(true);
                    controls.push(Control {
                        key: "ev_comp".into(),
                        label: "Compensación de exposición".into(),
                        group: Group::Exposure,
                        order: 4,
                        kind: Kind::Slider,
                        min: b.min,
                        max: b.max,
                        step: b.step,
                        default: b.default,
                        value: b.value,
                        source: Source::EvComp(bid),
                        inactive: !ae_auto,
                        read_only: false,
                        via: Some(Via::Driver),
                    });
                }
            }
            cam.read_led();
            cam.commands = [Command::Refocus, Command::AutoWhiteBalanceOnce, Command::AutoExposureOnce, Command::SaveToCamera, Command::FactoryReset, Command::Reboot]
                .into_iter()
                .map(|c| (c, cam.command_path(c)))
                .collect();
        }
        controls.sort_by_key(|c| (Group::ALL.iter().position(|g| *g == c.group).unwrap_or(9), c.order));
        cam.controls = controls;
        cam.refresh_readouts();
        cam.camera_profile = cam.read_camera_profile();
        cam.features = cam.build_features(&xu_status);
        Ok(cam)
    }

    /// Run an extension-unit request: the driver first, then direct USB.
    pub fn xu(&self, unit: u8, selector: u8, query: u8, buf: &mut [u8]) -> Result<Via, XuError> {
        let mut last = String::from("sin acceso");
        if let Some(dev) = &self.dev {
            match dev.xu_query(unit, selector, query, buf) {
                Ok(()) => return Ok(Via::Driver),
                Err(e) => last = e.to_string(),
            }
        }
        if let Some(usb) = &self.usb {
            return match usb.xu_query(unit, selector, query, buf) {
                Ok(()) => Ok(Via::Usb),
                Err(e) => Err(XuError::Failed(e.to_string())),
            };
        }
        if self.entry.elgato_webcam && !self.entry.advertised(unit, selector) && self.usb_error.is_some() {
            return Err(XuError::NoPermission);
        }
        Err(XuError::Failed(last))
    }

    /// Which path a one-shot command would take (write-only commands can't be read to probe).
    fn command_path(&self, cmd: Command) -> Option<Via> {
        let (unit, sel) = command_target(cmd);
        if self.entry.advertised(unit, sel) && self.dev.is_some() {
            Some(Via::Driver)
        } else if self.usb.is_some() {
            Some(Via::Usb)
        } else {
            None
        }
    }

    /// Length the camera declares for an extension-unit control (GET_LEN); `fallback` if it won't say.
    fn xu_len(&self, unit: u8, sel: u8, fallback: usize) -> usize {
        let mut len = [0u8; 2];
        match self.xu(unit, sel, v4l::UVC_GET_LEN, &mut len) {
            Ok(_) => match u16::from_le_bytes(len) as usize {
                0 => fallback,
                n => n.min(64),
            },
            Err(_) => fallback,
        }
    }

    /// Send "save profile" (XU2 selector 7 = 1, function 0x1802cb8b0). The payload is as long as
    /// the camera declares (the driver refuses any other size). `usb_first` skips the driver.
    fn send_save(&self, usb_first: bool) -> Result<Via, XuError> {
        let n = self.xu_len(elgato::XU2, elgato::PROFILE_SAVE_SELECTOR, 1);
        let mut payload = vec![0u8; n];
        payload[0] = elgato::PROFILE_SAVE_VALUE;
        let mut last = tr("sin acceso a la cámara");
        if !usb_first && self.entry.advertised(elgato::XU2, elgato::PROFILE_SAVE_SELECTOR) {
            if let Some(dev) = &self.dev {
                match dev.xu_query(elgato::XU2, elgato::PROFILE_SAVE_SELECTOR, v4l::UVC_SET_CUR, &mut payload) {
                    Ok(()) => return Ok(Via::Driver),
                    Err(e) => last = e.to_string(),
                }
            }
        }
        if let Some(usb) = &self.usb {
            payload.iter_mut().skip(1).for_each(|b| *b = 0);
            payload[0] = elgato::PROFILE_SAVE_VALUE;
            return usb
                .xu_query(elgato::XU2, elgato::PROFILE_SAVE_SELECTOR, v4l::UVC_SET_CUR, &mut payload)
                .map(|_| Via::Usb)
                .map_err(|e| XuError::Failed(e.to_string()));
        }
        if self.usb_error.is_some() {
            return Err(if self.entry.advertised(elgato::XU2, elgato::PROFILE_SAVE_SELECTOR) {
                XuError::Failed(last)
            } else {
                XuError::NoPermission
            });
        }
        Err(XuError::Failed(last))
    }

    /// Compare what the camera now keeps (its stored profile) with the live settings:
    /// (settings checked, matching, names that differ). None when the camera can't be read back.
    fn save_check(&self) -> Option<(usize, usize, Vec<String>)> {
        let rows = self.read_camera_profile()?;
        let (mut checked, mut matching, mut differ) = (0, 0, Vec::new());
        for r in &rows {
            let (Some((key, stored)), Some(now)) = (&r.target, r.now) else { continue };
            // Values the camera drives itself (auto exposure time, auto white balance…) move on their own.
            if self.find(key).map(|i| self.controls[i].inactive || self.controls[i].read_only).unwrap_or(true) {
                continue;
            }
            checked += 1;
            if *stored == now {
                matching += 1;
            } else {
                differ.push(tr(r.label));
            }
        }
        (checked > 0).then_some((checked, matching, differ))
    }

    /// Save the live settings inside the camera, like the original's "save as default", and read
    /// the camera's stored profile back to confirm it. If the stored values didn't change through
    /// the driver, it's sent again straight over USB.
    pub fn save_to_camera(&mut self) -> Result<String, String> {
        if self.dev.is_none() {
            return Err(tr("sin acceso a la cámara"));
        }
        // 0x1802cb8b0: pending throttled writes go out first, then selector 7 = 1.
        let _ = self.flush();
        let mut via = self.send_save(false).map_err(|e| tr_n("No se pudo: {}", e))?;
        std::thread::sleep(std::time::Duration::from_millis(elgato::SAVE_SETTLE_MS));
        let mut check = self.save_check();
        if via == Via::Driver && self.usb.is_some() && check.as_ref().map(|c| !c.2.is_empty()).unwrap_or(false) {
            if let Ok(v) = self.send_save(true) {
                via = v;
                std::thread::sleep(std::time::Duration::from_millis(elgato::SAVE_SETTLE_MS));
                check = self.save_check();
            }
        }
        let _ = via;
        self.camera_profile = self.read_camera_profile();
        Ok(match check {
            None => tr("La cámara aceptó la orden de guardar (no deja comprobar qué guardó)"),
            Some((n, m, d)) if d.is_empty() => trf("Guardado dentro de la cámara: comprobados {} de {} ajustes", &[&m, &n]),
            Some((n, m, d)) => trf("La cámara guardó {} de {} ajustes; distintos: {}", &[&m, &n, &d.join(", ")]),
        })
    }

    pub fn run_command(&mut self, cmd: Command) -> Result<String, String> {
        if cmd == Command::SaveToCamera {
            return self.save_to_camera();
        }
        let (unit, sel) = command_target(cmd);
        let mut payload: Vec<u8> = match cmd {
            Command::Refocus => elgato::ONE_TIME_AUTO_FOCUS.to_le_bytes().to_vec(),
            Command::AutoWhiteBalanceOnce => elgato::ONE_TIME_AUTO_WHITE_BALANCE.to_le_bytes().to_vec(),
            Command::AutoExposureOnce => elgato::ONE_TIME_AUTO_EXPOSURE.to_le_bytes().to_vec(),
            Command::SaveToCamera => vec![elgato::PROFILE_SAVE_VALUE],
            // ResetToFactoryDefaults (0x1802cb6e0): load profile 0 (factory), then save it.
            Command::FactoryReset => vec![elgato::PROFILE_FACTORY],
            Command::Reboot => vec![0],
        };
        // The driver only accepts the exact length the camera declares (GET_LEN).
        if matches!(cmd, Command::FactoryReset | Command::Reboot) {
            let n = self.xu_len(unit, sel, payload.len());
            payload.resize(n, 0);
        }
        // Through the driver only when listed (unlisted selectors fail there); otherwise USB.
        let res = if self.entry.advertised(unit, sel) {
            self.xu(unit, sel, v4l::UVC_SET_CUR, &mut payload)
        } else if let Some(usb) = &self.usb {
            usb.xu_query(unit, sel, v4l::UVC_SET_CUR, &mut payload).map(|_| Via::Usb).map_err(|e| XuError::Failed(e.to_string()))
        } else if self.usb_error.is_some() {
            Err(XuError::NoPermission)
        } else {
            Err(XuError::Failed(tr("sin acceso a la cámara")))
        };
        match res {
            Ok(_) => {
                if cmd == Command::FactoryReset {
                    let _ = self.send_save(false);
                    std::thread::sleep(std::time::Duration::from_millis(elgato::SAVE_SETTLE_MS));
                    self.reread_all();
                    self.camera_profile = self.read_camera_profile();
                }
                Ok(tr(match cmd {
                    Command::Refocus => "Reenfocando…",
                    Command::AutoWhiteBalanceOnce => "Balance de blancos ajustado una vez",
                    Command::AutoExposureOnce => "Exposición ajustada una vez",
                    Command::SaveToCamera => "Ajustes guardados dentro de la cámara",
                    Command::FactoryReset => "Cámara restablecida a valores de fábrica",
                    Command::Reboot => "La cámara se está reiniciando: espera unos segundos y pulsa actualizar",
                }
                ))
            }
            Err(e) => Err(tr_n("No se pudo: {}", e)),
        }
    }

    fn select_led(&self, index: u8) -> Result<Via, XuError> {
        let mut sel = vec![0u8; elgato::UVC_INDEX_LEN];
        sel[0] = elgato::LED_SELECTOR;
        sel[7] = index;
        self.xu(elgato::XU2, elgato::UVC_INDEX_SELECTOR, v4l::UVC_SET_CUR, &mut sel)
    }

    fn led_raw(&self, index: u8) -> Result<(Via, Vec<u8>), XuError> {
        self.select_led(index)?;
        let mut buf = vec![0u8; elgato::LED_LEN];
        let via = self.xu(elgato::XU2, elgato::LED_SELECTOR, v4l::UVC_GET_CUR, &mut buf)?;
        Ok((via, buf))
    }

    pub fn read_led(&mut self) {
        let s = self.led_raw(elgato::LED_INDEX_STREAMING);
        let i = self.led_raw(elgato::LED_INDEX_IDLE);
        if let (Ok((via, s)), Ok((_, i))) = (s, i) {
            let sr = s[elgato::LED_BRIGHTNESS_BYTE] as i32;
            let ir = i[elgato::LED_BRIGHTNESS_BYTE] as i32;
            let raw = sr.max(ir);
            self.led = Some(Led { mode: elgato::led_mode(sr, ir), percent: if raw == 0 { 100 } else { raw * 100 / elgato::LED_RAW_MAX } });
            self.led_via = Some(via);
        }
    }

    /// Write the LED as the original does (0x1802c6a40): the whole 10-byte LED_CONTROL for the
    /// streaming slot, then for the idle slot; then read it back.
    pub fn set_led(&mut self, led: Led) -> Result<(), String> {
        let raw = (led.percent.clamp(0, 100) * elgato::LED_RAW_MAX / 100) as u8;
        for (index, on) in [(elgato::LED_INDEX_STREAMING, led.mode >= 1), (elgato::LED_INDEX_IDLE, led.mode == 2)] {
            let mut buf = elgato::led_write_payload(index, if on { raw } else { 0 }).to_vec();
            self.xu(elgato::XU2, elgato::LED_SELECTOR, v4l::UVC_SET_CUR, &mut buf).map_err(|e| e.to_string())?;
        }
        self.read_led();
        Ok(())
    }

    fn build_features(&self, xu: &BTreeMap<&'static str, Result<Via, bool>>) -> Vec<Feature> {
        let has_std = |id: u32| self.controls.iter().any(|c| matches!(c.source, Source::Std(i) if i == id));
        let std = |id: u32| if has_std(id) { Status::Driver } else { Status::NotOnCamera };
        let xs = |key: &str| match xu.get(key) {
            Some(Ok(Via::Driver)) => Status::Driver,
            Some(Ok(Via::Usb)) => Status::Usb,
            Some(Err(true)) => Status::NeedsPermission,
            Some(Err(false)) => Status::NotOnCamera,
            None => Status::NotOnCamera,
        };
        let cmd = |c: Command| match self.commands.iter().find(|x| x.0 == c).and_then(|x| x.1) {
            Some(Via::Driver) => Status::Driver,
            Some(Via::Usb) => Status::Usb,
            None if self.usb_error.is_some() => Status::NeedsPermission,
            None => Status::NotOnCamera,
        };
        let fps: Vec<String> = {
            let mut v: Vec<f32> = self.formats.iter().flat_map(|f| f.intervals.iter().flatten().map(|iv| v4l::fps_value(*iv))).collect();
            v.sort_by(|a, b| b.partial_cmp(a).unwrap());
            v.dedup_by(|a, b| (*a - *b).abs() < 0.5);
            v.iter().map(|f| format!("{f:.0}")).collect()
        };
        let led = match (self.led_via, self.usb_error.is_some()) {
            (Some(Via::Driver), _) => Status::Driver,
            (Some(Via::Usb), _) => Status::Usb,
            (None, true) => Status::NeedsPermission,
            (None, false) => Status::NotOnCamera,
        };
        let f = |name, section, status, note: &str| Feature { name, section, status, note: note.to_string() };
        vec![
            f("Vista previa", "Captura", if self.formats.is_empty() { Status::NotOnCamera } else { Status::Driver }, "MJPEG, YUY2 o NV12"),
            f("Resolución y fotogramas por segundo", "Captura", if fps.is_empty() { Status::NotOnCamera } else { Status::Driver }, &tr_n("{} fps disponibles", fps.join(" / "))),
            f("Foto", "Captura", Status::App, "PNG en Imágenes/Lente"),
            f("Zoom", "Encuadre", std(elgato::CID_ZOOM), ""),
            f("Desplazamiento (pan)", "Encuadre", std(elgato::CID_PAN), ""),
            f("Inclinación (tilt)", "Encuadre", std(elgato::CID_TILT), ""),
            f("Encuadres guardados (4)", "Encuadre", Status::App, "guardados por Lente, como en el original"),
            f("Transición suave entre encuadres", "Encuadre", Status::App, ""),
            f("Pantalla completa y cuadrícula", "Captura", Status::App, "F11 / Esc"),
            f("Voltear / espejo", "Encuadre", xs("xu_flip"), ""),
            f("Exposición automática", "Exposición", std(elgato::CID_EXPOSURE_AUTO), ""),
            f("Compensación de exposición", "Exposición", if has_std(elgato::CID_BRIGHTNESS) { Status::Driver } else { Status::NotOnCamera }, "mueve el brillo con exposición automática"),
            f("Velocidad de obturación", "Exposición", std(0x009a_0902), ""),
            f("ISO / ganancia", "Exposición", std(0x0098_0913), ""),
            f("Medición de exposición", "Exposición", xs("xu_ae_metering"), ""),
            f("Bloquear exposición", "Exposición", xs("xu_ae_lock"), ""),
            f("Antiparpadeo", "Exposición", std(0x0098_0918), ""),
            f("Modo poca luz", "Exposición", xs("xu_binning"), "Apagado / <720p / <1080p"),
            f("Enfoque automático", "Enfoque", std(elgato::CID_FOCUS_AUTO), ""),
            f("Distancia de enfoque", "Enfoque", std(elgato::CID_FOCUS_ABS), ""),
            f("Medición de enfoque", "Enfoque", xs("xu_af_metering"), ""),
            f("Bloquear enfoque", "Enfoque", xs("xu_af_lock"), ""),
            f("Reenfocar ahora", "Enfoque", cmd(Command::Refocus), ""),
            f("Preajustes de enfoque (4)", "Enfoque", if has_std(elgato::CID_FOCUS_ABS) { Status::App } else { Status::NotOnCamera }, ""),
            f("Balance de blancos automático", "Balance de blancos", std(0x0098_090c), ""),
            f("Temperatura", "Balance de blancos", std(0x0098_091a), ""),
            f("Tono", "Balance de blancos", xs("xu_tint"), ""),
            f("Bloquear balance de blancos", "Balance de blancos", xs("xu_awb_lock"), ""),
            f("Brillo", "Imagen", std(elgato::CID_BRIGHTNESS), ""),
            f("Contraste", "Imagen", std(0x0098_0901), ""),
            f("Saturación", "Imagen", std(0x0098_0902), ""),
            f("Intensidad de color", "Imagen", xs("xu_vibrancy"), ""),
            f("Rango dinámico (HDR)", "Imagen", xs("xu_hdr"), &trf("HDR hasta {} fps ({} en algunos modelos); Lente baja los fps al activarlo", &[&elgato::HDR_MAX_FPS, &elgato::HDR_MAX_FPS_EXTENDED])),
            f("Nitidez", "Detalle", std(0x0098_091b), ""),
            f("Reducción de ruido", "Detalle", xs("xu_nr"), ""),
            f("Reducción de ruido 2D", "Detalle", xs("xu_nr2d"), ""),
            f("Reducción de ruido 3D", "Detalle", xs("xu_nr3d"), ""),
            f("Luz LED de estado", "Dispositivo", led, "experimental"),
            f("Guardar ajustes en la cámara", "Dispositivo", cmd(Command::SaveToCamera), "se conservan en otros equipos y programas"),
            f("Usar los ajustes guardados en Windows", "Dispositivo", match &self.camera_profile { Some(_) => Status::Driver, None if self.dev.is_none() => Status::Pending, None => Status::NotOnCamera }, "lee el perfil interno de la cámara"),
            f("Restablecer de fábrica", "Dispositivo", cmd(Command::FactoryReset), ""),
            f("Reiniciar la cámara", "Dispositivo", cmd(Command::Reboot), ""),
            f("Actualizar firmware", "Dispositivo", Status::WindowsOnly("el firmware no viene en el paquete y un fallo puede inutilizar la cámara"), ""),
            f("Grabación de vídeo", "Otras funciones", Status::WindowsOnly("usa OBS Studio en Linux"), ""),
            f("Teleprompter y chat", "Otras funciones", Status::WindowsOnly("fuera del control de cámara"), ""),
        ]
    }

    fn demo(entry: CameraEntry) -> Camera {
        // Valores de ejemplo para la vista sin cámara (rangos estándar estimados).
        let mk = |key: &str, label: &str, group, order, kind: Kind, min, max, def, value| Control {
            key: key.into(),
            label: label.into(),
            group,
            order,
            kind,
            min,
            max,
            step: 1,
            default: def,
            value,
            source: Source::Demo,
            inactive: false,
            read_only: false,
            via: None,
        };
        let onoff = || Kind::Toggle;
        let mut controls = vec![
            mk("zoom", "Zoom", Group::Frame, 0, Kind::Slider, 100, 400, 100, 140),
            mk("pan", "Desplazamiento horizontal (pan)", Group::Frame, 1, Kind::Slider, -36000, 36000, 0, 0),
            mk("tilt", "Inclinación (tilt)", Group::Frame, 2, Kind::Slider, -36000, 36000, 0, 0),
            mk("ae", "Exposición automática", Group::Exposure, 0, Kind::Menu(vec![(1, "Manual".into()), (3, "Automática (prioridad de apertura)".into())]), 0, 3, 3, 3),
            mk("ev", "Compensación de exposición", Group::Exposure, 4, Kind::Slider, -9, 9, 0, 0),
            mk("lowlight", "Modo poca luz", Group::Exposure, 20, low_light_menu(), 0, 62, 0, 0),
            mk("plf", "Antiparpadeo", Group::Exposure, 5, Kind::Menu(vec![(0, "Desactivado".into()), (1, "50 Hz".into()), (2, "60 Hz".into())]), 0, 2, 2, 2),
            mk("af", "Enfoque automático", Group::Focus, 0, onoff(), 0, 1, 1, 1),
            mk("focus", "Distancia de enfoque", Group::Focus, 1, Kind::Slider, 0, 255, 128, 96),
            mk("awb", "Balance de blancos automático", Group::WhiteBalance, 0, onoff(), 0, 1, 1, 0),
            mk("wb", "Temperatura (K)", Group::WhiteBalance, 1, Kind::Slider, 2800, 7500, 5000, 4600),
            mk("tint", "Tono (verde ↔ magenta)", Group::WhiteBalance, 2, Kind::Slider, -300, 300, 0, 0),
            mk("bri", "Brillo", Group::Picture, 0, Kind::Slider, -9, 9, 0, 0),
            mk("con", "Contraste", Group::Picture, 1, Kind::Slider, 0, 100, 50, 50),
            mk("sat", "Saturación", Group::Picture, 2, Kind::Slider, 0, 127, 64, 64),
            mk("vib", "Intensidad de color", Group::Picture, 3, Kind::Slider, 0, 100, 0, 20),
            mk("dr", "Rango dinámico", Group::Picture, 10, Kind::Menu(vec![(0, "Estándar".into()), (1, "Amplio".into()), (2, "Alto (HDR)".into())]), 0, 2, 0, 0),
            mk("sharp", "Nitidez", Group::Detail, 0, Kind::Slider, 0, 4, 2, 2),
            mk("nr3d", "Reducción de ruido 3D", Group::Detail, 2, Kind::Slider, 0, 100, 50, 50),
        ];
        controls.sort_by_key(|c| (Group::ALL.iter().position(|g| *g == c.group).unwrap_or(9), c.order));
        let sizes = vec![(3840, 2160), (2560, 1440), (1920, 1080), (1280, 720), (960, 540), (640, 360)];
        let intervals = sizes.iter().map(|_| vec![(1, 60), (1, 30)]).collect();
        let mut cam = Camera {
            entry,
            dev: None,
            usb: None,
            usb_error: None,
            controls,
            formats: vec![v4l::FormatInfo { fourcc: v4l::FMT_MJPG, description: "Motion-JPEG".into(), sizes, intervals }],
            skipped_xu: vec![],
            readouts: vec![("Ganancia actual".into(), Some(12)), ("Temperatura actual (K)".into(), Some(4630))],
            sensor: None,
            features: vec![],
            led: Some(Led { mode: 1, percent: 100 }),
            led_via: None,
            commands: vec![],
            camera_profile: None,
            pending: BTreeMap::new(),
            last_write: BTreeMap::new(),
        };
        cam.features = cam.build_features(&BTreeMap::new()).into_iter().map(|mut f| {
            if !matches!(f.status, Status::WindowsOnly(_) | Status::App) {
                f.status = Status::Pending;
            }
            f
        }).collect();
        cam
    }

    pub fn refresh_readouts(&mut self) {
        if self.dev.is_none() || !self.entry.elgato_webcam {
            return;
        }
        let mut out = Vec::new();
        for (label, unit, sel) in elgato::XU_READOUTS {
            if !self.entry.advertised(*unit, *sel) {
                continue; // don't poll over USB every second
            }
            let mut buf = [0u8; 4];
            if self.xu(*unit, *sel, v4l::UVC_GET_CUR, &mut buf).is_ok() {
                out.push((label.to_string(), Some(u32::from_le_bytes(buf) as i64)));
            }
        }
        self.readouts = out;
        self.sensor = self.camera_state();
    }

    /// CElgatoWebcamBaseSupport::GetCameraState (0x180261db0): GET_CUR of 13 bytes on XU1
    /// selector 1, laid out as CAMERA_STATE (SWIG getters 0x1800d5560..0x1800d55e0).
    pub fn camera_state(&self) -> Option<CameraState> {
        if !self.entry.advertised(elgato::XU1, elgato::CAMERA_STATUS_SELECTOR) {
            return None;
        }
        let mut b = vec![0u8; elgato::CAMERA_STATUS_LEN];
        self.xu(elgato::XU1, elgato::CAMERA_STATUS_SELECTOR, v4l::UVC_GET_CUR, &mut b).ok()?;
        Some(elgato::parse_camera_state(&b))
    }

    /// Write a value; returns an error message for the status bar.
    /// Original property id behind a control, for the rules that are kept per property.
    fn original_prop(&self, idx: usize) -> Option<i32> {
        match &self.controls[idx].source {
            Source::Std(elgato::CID_FOCUS_ABS) => Some(elgato::PROP_CT_FOCUS),
            Source::Std(elgato::CID_ZOOM) => Some(elgato::PROP_CT_ZOOM),
            Source::Std(elgato::CID_PAN) => Some(elgato::PROP_CT_PAN),
            Source::Std(elgato::CID_TILT) => Some(elgato::PROP_CT_TILT),
            Source::Std(elgato::CID_WB_TEMP) => Some(elgato::PROP_PU_WHITE_BALANCE_TEMP),
            Source::Xu(def) if def.key == "xu_tint" => Some(elgato::PROP_XU_WHITE_BALANCE_TINT),
            _ => None,
        }
    }

    /// Change a setting. Focus, zoom, pan, tilt, temperature and tint are spaced 50 ms apart like
    /// the original (a dragged slider would otherwise flood the camera); the newest value is
    /// kept and sent by pump() or flush().
    pub fn set(&mut self, idx: usize, value: i32) -> Result<(), String> {
        let throttled = self.original_prop(idx).map(|p| elgato::THROTTLED_PROPS.contains(&p)).unwrap_or(false);
        if throttled && self.dev.is_some() {
            let c = &self.controls[idx];
            let value = value.clamp(c.min.min(c.max), c.max.max(c.min));
            let recent = self
                .last_write
                .get(&idx)
                .map(|t| t.elapsed() < std::time::Duration::from_millis(elgato::THROTTLE_MS))
                .unwrap_or(false);
            if recent {
                self.pending.insert(idx, value);
                self.controls[idx].value = value;
                return Ok(());
            }
            self.pending.remove(&idx);
            self.last_write.insert(idx, std::time::Instant::now());
        }
        self.write_now(idx, value)
    }

    /// Whether throttled values are still waiting (the window keeps drawing until they're sent).
    pub fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }

    /// Send throttled values whose 50 ms have passed (Throttler timing loop 0x180274500).
    pub fn pump(&mut self) -> Vec<String> {
        let due: Vec<(usize, i32)> = self
            .pending
            .iter()
            .filter(|(i, _)| {
                self.last_write
                    .get(i)
                    .map(|t| t.elapsed() >= std::time::Duration::from_millis(elgato::THROTTLE_MS))
                    .unwrap_or(true)
            })
            .map(|(i, v)| (*i, *v))
            .collect();
        let mut errs = Vec::new();
        for (i, v) in due {
            self.pending.remove(&i);
            self.last_write.insert(i, std::time::Instant::now());
            if let Err(e) = self.write_now(i, v) {
                errs.push(e);
            }
        }
        errs
    }

    /// Send every waiting value now (Throttler::Flush 0x180273d40, called before saving).
    pub fn flush(&mut self) -> Vec<String> {
        let all: Vec<(usize, i32)> = std::mem::take(&mut self.pending).into_iter().collect();
        let mut errs = Vec::new();
        for (i, v) in all {
            self.last_write.insert(i, std::time::Instant::now());
            if let Err(e) = self.write_now(i, v) {
                errs.push(e);
            }
        }
        errs
    }

    fn write_now(&mut self, idx: usize, value: i32) -> Result<(), String> {
        let c = &self.controls[idx];
        let value = value.clamp(c.min.min(c.max), c.max.max(c.min));
        let res = match (&c.source, &self.dev) {
            (Source::Demo, _) => Ok(()),
            (_, None) => Err("sin dispositivo".to_string()),
            (Source::Std(id), Some(dev)) => dev.set_ctrl(*id, value).map_err(|e| e.to_string()),
            (Source::Xu(def), Some(_)) => {
                let mut b = elgato::encode(def, value);
                self.xu(def.unit, def.selector, v4l::UVC_SET_CUR, &mut b).map(|_| ()).map_err(|e| e.to_string())
            }
            (Source::EvComp(bid), Some(dev)) => {
                let ae_manual = self
                    .controls
                    .iter()
                    .find(|c| matches!(c.source, Source::Std(elgato::CID_EXPOSURE_AUTO)))
                    .map(|c| c.value == 1)
                    .unwrap_or(false);
                if ae_manual {
                    Err(tr("activa primero la exposición automática"))
                } else {
                    dev.set_ctrl(*bid, value).map_err(|e| e.to_string())
                }
            }
            (Source::DynamicRange { hdr, blc }, Some(dev)) => {
                let (hdr_v, blc_v) = elgato::dynamic_range_writes(value);
                let mut b = elgato::encode(hdr, hdr_v);
                let r = self.xu(hdr.unit, hdr.selector, v4l::UVC_SET_CUR, &mut b).map(|_| ()).map_err(|e| e.to_string());
                if let (Ok(()), Some(id)) = (&r, blc) {
                    let _ = dev.set_ctrl(*id, blc_v);
                }
                r
            }
        };
        match res {
            Ok(()) => {
                self.controls[idx].value = value;
                self.refresh_after_write();
                Ok(())
            }
            Err(e) => {
                let label = self.controls[idx].label.clone();
                self.reread(idx);
                Err(trf("{}: la cámara rechazó el valor ({})", &[&tr(&label), &e]))
            }
        }
    }

    fn reread(&mut self, idx: usize) {
        if self.dev.is_none() {
            return;
        }
        let source = self.controls[idx].source.clone();
        let v = match &source {
            Source::Std(id) => self.dev.as_ref().unwrap().get_ctrl(*id).ok(),
            Source::Xu(def) => {
                let mut b = vec![0u8; def.size as usize];
                self.xu(def.unit, def.selector, v4l::UVC_GET_CUR, &mut b).ok().map(|_| xu_display_value(def, elgato::decode(def, &b)))
            }
            _ => None,
        };
        if let Some(v) = v {
            self.controls[idx].value = v;
        }
    }

    pub fn reread_all(&mut self) {
        for i in 0..self.controls.len() {
            self.reread(i);
        }
        self.refresh_after_write();
    }

    /// Some controls turn inactive when an "auto" switch changes; re-read flags and values.
    pub fn refresh_after_write(&mut self) {
        let Some(dev) = &self.dev else { return };
        let ae_auto = self
            .controls
            .iter()
            .find(|c| matches!(c.source, Source::Std(elgato::CID_EXPOSURE_AUTO)))
            .map(|c| c.value != 1)
            .unwrap_or(true);
        for c in &mut self.controls {
            if let Source::EvComp(bid) = c.source {
                c.inactive = !ae_auto;
                if let Ok(v) = dev.get_ctrl(bid) {
                    c.value = v;
                }
            }
            if let Source::Std(id) = c.source {
                if let Some(f) = dev.ctrl_flags(id) {
                    c.inactive = f & v4l::CTRL_FLAG_INACTIVE != 0;
                }
                if c.kind != Kind::Button {
                    if let Ok(v) = dev.get_ctrl(id) {
                        c.value = v;
                    }
                }
            }
        }
    }

    pub fn reset_all(&mut self) -> Vec<String> {
        let mut errs = Vec::new();
        // Auto switches first so dependent manual values are accepted afterwards.
        let mut order: Vec<usize> = (0..self.controls.len()).collect();
        order.sort_by_key(|&i| if matches!(self.controls[i].kind, Kind::Toggle | Kind::Menu(_)) { 0 } else { 1 });
        for i in order {
            let c = &self.controls[i];
            if c.read_only || c.kind == Kind::Button || c.value == c.default || matches!(c.source, Source::EvComp(_)) {
                continue;
            }
            if let Err(e) = self.set(i, self.controls[i].default) {
                errs.push(e);
            }
        }
        errs
    }

    pub fn snapshot_values(&self) -> BTreeMap<String, i32> {
        self.controls
            .iter()
            .filter(|c| !c.read_only && c.kind != Kind::Button && !matches!(c.source, Source::EvComp(_)))
            .map(|c| (c.key.clone(), c.value))
            .collect()
    }

    pub fn apply_values(&mut self, values: &BTreeMap<String, i32>) -> Vec<String> {
        let mut errs = Vec::new();
        let mut order: Vec<usize> = (0..self.controls.len()).collect();
        order.sort_by_key(|&i| if matches!(self.controls[i].kind, Kind::Toggle | Kind::Menu(_)) { 0 } else { 1 });
        for i in order {
            let key = self.controls[i].key.clone();
            if let Some(&v) = values.get(&key) {
                if self.controls[i].value != v && !self.controls[i].read_only {
                    if self.controls[i].inactive {
                        continue;
                    }
                    if let Err(e) = self.set(i, v) {
                        errs.push(e);
                    }
                }
            }
        }
        errs
    }

    pub fn find(&self, key: &str) -> Option<usize> {
        self.controls.iter().position(|c| c.key == key)
    }
    pub fn find_std(&self, id: u32) -> Option<usize> {
        self.controls.iter().position(|c| matches!(c.source, Source::Std(i) if i == id))
    }
}

/// One setting stored in the camera's own profile, next to the value it has now.
#[derive(Clone, Debug)]
pub struct ProfileRow {
    pub label: &'static str,
    pub stored: i64,
    /// Our control key and the value to write (converted from the camera's units).
    pub target: Option<(String, i32)>,
    pub now: Option<i32>,
}

impl Camera {
    /// XU2 selector 8 holds the camera's saved profile as JSON ({"Brightness":{"value":3},...}).
    /// It is what Elgato's Windows app saved with USER_PROFILE_SAVE (selector 7), so applying
    /// it makes the picture look the way it does on Windows.
    pub fn read_camera_profile(&self) -> Option<Vec<ProfileRow>> {
        if self.dev.is_none() || !self.entry.elgato_webcam {
            return None;
        }
        let mut len = [0u8; 2];
        self.xu(elgato::XU2, elgato::PROFILE_JSON_SELECTOR, v4l::UVC_GET_LEN, &mut len).ok()?;
        let n = u16::from_le_bytes(len) as usize;
        if n == 0 || n > 65535 {
            return None;
        }
        let mut buf = vec![0u8; n];
        self.xu(elgato::XU2, elgato::PROFILE_JSON_SELECTOR, v4l::UVC_GET_CUR, &mut buf).ok()?;
        let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
        let json: serde_json::Value = serde_json::from_slice(&buf[..end]).ok()?;
        let obj = json.as_object()?;
        let mut rows = Vec::new();
        for (key, label, target) in elgato::PROFILE_KEYS {
            let Some(v) = obj.get(*key).and_then(|e| e.get("value")).and_then(|v| v.as_i64()) else { continue };
            let t = match target {
                elgato::ProfileTarget::Std(id) => self.find_std(*id).map(|i| (self.controls[i].key.clone(), v as i32)),
                elgato::ProfileTarget::StdAeMode => self.find_std(elgato::CID_EXPOSURE_AUTO).and_then(|i| {
                    elgato::uvc_ae_mode_to_v4l2(v as i32).map(|m| (self.controls[i].key.clone(), m))
                }),
                elgato::ProfileTarget::Xu(k) => self.find(k).map(|i| {
                    let val = if *k == "xu_binning" { elgato::low_light_from_profile(v as i32) } else { v as i32 };
                    (self.controls[i].key.clone(), val)
                }),
            };
            let now = t.as_ref().and_then(|(k, _)| self.find(k).map(|i| self.controls[i].value));
            rows.push(ProfileRow { label, stored: v, target: t, now });
        }
        if rows.is_empty() {
            None
        } else {
            Some(rows)
        }
    }

    pub fn apply_camera_profile(&mut self) -> Vec<String> {
        let Some(rows) = self.read_camera_profile() else { return vec![tr("La cámara no tiene un perfil guardado legible")] };
        let values: BTreeMap<String, i32> = rows.iter().filter_map(|r| r.target.clone()).collect();
        let errs = self.apply_values(&values);
        self.camera_profile = self.read_camera_profile();
        errs
    }
}

fn command_target(cmd: Command) -> (u8, u8) {
    match cmd {
        Command::Refocus | Command::AutoWhiteBalanceOnce | Command::AutoExposureOnce => (elgato::XU1, elgato::ONE_TIME_AUTO_SELECTOR),
        Command::SaveToCamera => (elgato::XU2, elgato::PROFILE_SAVE_SELECTOR),
        Command::FactoryReset => (elgato::XU2, elgato::PROFILE_LOAD_SELECTOR),
        Command::Reboot => (elgato::XU2, elgato::REBOOT_SELECTOR),
    }
}

fn low_light_menu() -> Kind {
    Kind::Menu(vec![
        (elgato::LOW_LIGHT_OFF, "Apagado".to_string()),
        (elgato::LOW_LIGHT_BELOW_720P, "Por debajo de 720p".to_string()),
        (elgato::LOW_LIGHT_BELOW_1080P, "Por debajo de 1080p".to_string()),
    ])
}

/// Raw XU value -> what the menu shows (low light uses the original's BitfieldToIndex rule).
fn xu_display_value(def: &XuDef, raw: i32) -> i32 {
    if def.key == "xu_binning" {
        elgato::low_light_from_bitfield(raw)
    } else {
        raw
    }
}

fn xu_control(def: &XuDef, value: i32) -> Control {
    let kind = if def.key == "xu_binning" {
        low_light_menu()
    } else if !def.menu.is_empty() {
        Kind::Menu(def.menu.iter().enumerate().map(|(i, s)| (i as i32, s.to_string())).collect())
    } else if def.min == 0 && def.max == 1 {
        Kind::Toggle
    } else {
        Kind::Slider
    };
    Control {
        key: def.key.into(),
        label: def.label.into(),
        group: def.group,
        order: 20,
        kind,
        min: def.min,
        max: def.max,
        step: def.step,
        default: def.default,
        value,
        source: Source::Xu(*def),
        inactive: false,
        read_only: false,
        via: None,
    }
}

// ---------------------------------------------------------------- profiles

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct FramePreset {
    pub name: String,
    pub values: BTreeMap<String, i32>,
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct Profile {
    pub apply_on_open: bool,
    pub defaults: Option<BTreeMap<String, i32>>,
    pub frame_presets: Vec<Option<FramePreset>>,
    /// (fourcc, width, height, interval numerator, denominator)
    #[serde(default)]
    pub video_mode: Option<(u32, u32, u32, u32, u32)>,
    /// Apply the profile stored in the camera (what Windows saved) when opening it.
    #[serde(default = "yes")]
    pub use_camera_profile: bool,
    #[serde(default)]
    pub focus_presets: Vec<Option<i32>>,
    #[serde(default = "yes")]
    pub smooth_transitions: bool,
}

fn yes() -> bool {
    true
}

pub fn config_dir() -> PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("lente-camara")
}

pub fn load_profile(id: &str) -> Profile {
    let p = config_dir().join(format!("{id}.json"));
    let mut prof: Profile = std::fs::read_to_string(p).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or(Profile {
        apply_on_open: true,
        use_camera_profile: true,
        smooth_transitions: true,
        ..Default::default()
    });
    prof.frame_presets.resize(elgato::FRAME_PRESET_COUNT, None);
    prof
}

pub fn save_profile(id: &str, prof: &Profile) -> Result<PathBuf, String> {
    let dir = config_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let p = dir.join(format!("{id}.json"));
    let s = serde_json::to_string_pretty(prof).map_err(|e| e.to_string())?;
    std::fs::write(&p, s).map_err(|e| e.to_string())?;
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn xu_descriptor_parse() {
        // Facecam 4K unit 6: 12 controls, 1 input pin, control size 4, bitmap e7 07 91 00
        let mut d = vec![28u8, 0x24, 0x06, 6];
        d.extend_from_slice(&[0xd0, 0x9e, 0xe4, 0x23, 0x78, 0x11, 0x31, 0x4f, 0xae, 0x52, 0xd2, 0xfb, 0x8a, 0x8d, 0x30, 0x01]);
        d.extend_from_slice(&[12, 1, 3, 4, 0xe7, 0x07, 0x91, 0x00]);
        let u = parse_xu_units(&d);
        assert_eq!(u[0].id, 6);
        assert_eq!(u[0].guid, "23e49ed0-1178-4f31-ae52-d2fb8a8d3001");
        assert_eq!(u[0].selectors, vec![1, 2, 3, 6, 7, 8, 9, 10, 11, 17, 21, 24]);
    }
}
