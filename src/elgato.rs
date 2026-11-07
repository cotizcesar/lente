//! Facts about Elgato devices, taken from the original program (see MECHANICS.md).

pub const ELGATO_VENDOR: u16 = 0x0fd9;

/// (product id, model name, is a webcam with Elgato extension units)
pub const MODELS: &[(u16, &str, bool)] = &[
    (0x77, "Facecam (USB 2.0)", true),
    (0x78, "Facecam", true),
    (0x87, "Facecam Pro (USB 2.0)", true),
    (0x79, "Facecam Pro", true),
    (0x81, "Facecam Neo", true),
    (0x91, "Facecam 4K (USB 2.0)", true),
    (0x92, "Facecam 4K", true),
    (0x93, "Facecam MK.2 (USB 2.0)", true),
    (0x94, "Facecam MK.2", true),
    (0xd6, "Facecam MK.2 Rev.2 (USB 2.0)", true),
    (0xd7, "Facecam MK.2 Rev.2", true),
    (0x61, "Cam Link", false),
    (0x66, "Cam Link 4K", false),
    (0x7b, "Cam Link 4K", false),
    (0xa1, "Cam Link 4K", false),
    (0xa2, "Cam Link 4K", false),
    (0xf4, "Cam Link 4K MK.2", false),
    (0xf5, "Cam Link 4K MK.2", false),
    (0x6a, "Game Capture HD60 S+", false),
    (0x82, "Game Capture HD60 X", false),
    (0x8a, "Game Capture HD60 X Rev.2", false),
    (0x8c, "Game Capture Neo", false),
    (0xaf, "Game Capture 4K S", false),
    (0xdc, "Game Capture 4K S", false),
];

pub fn model(vendor: u16, product: u16) -> Option<(&'static str, bool)> {
    if vendor != ELGATO_VENDOR {
        return None;
    }
    MODELS.iter().find(|m| m.0 == product).map(|m| (m.1, m.2))
}

/// Extension unit numbers used by the original: 0x02060000 → unit 6, 0x02070000 → unit 7.
pub const XU1: u8 = 6;
pub const XU2: u8 = 7;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Group {
    Picture,
    Exposure,
    WhiteBalance,
    Focus,
    Frame,
    Detail,
    Advanced,
}

impl Group {
    pub const ALL: [Group; 7] = [
        Group::Frame,
        Group::Exposure,
        Group::Focus,
        Group::WhiteBalance,
        Group::Picture,
        Group::Detail,
        Group::Advanced,
    ];
    pub fn title(self) -> &'static str {
        match self {
            Group::Picture => "Imagen",
            Group::Exposure => "Exposición",
            Group::WhiteBalance => "Balance de blancos",
            Group::Focus => "Enfoque",
            Group::Frame => "Encuadre",
            Group::Detail => "Detalle",
            Group::Advanced => "Avanzado",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct XuDef {
    pub key: &'static str,
    pub label: &'static str,
    pub group: Group,
    pub unit: u8,
    pub selector: u8,
    pub size: u8,
    pub signed: bool,
    pub min: i32,
    pub max: i32,
    pub step: i32,
    pub default: i32,
    pub menu: &'static [&'static str],
}

/// Writable extension-unit controls from the descriptor table at 0x180662550.
pub const XU_CONTROLS: &[XuDef] = &[
    XuDef { key: "xu_tint", label: "Tono (verde ↔ magenta)", group: Group::WhiteBalance, unit: XU1, selector: 18, size: 2, signed: true, min: -300, max: 300, step: 1, default: 0, menu: &[] },
    XuDef { key: "xu_vibrancy", label: "Intensidad de color", group: Group::Picture, unit: XU1, selector: 14, size: 2, signed: false, min: 0, max: 100, step: 1, default: 0, menu: &[] },
    XuDef { key: "xu_hdr", label: "HDR", group: Group::Picture, unit: XU1, selector: 21, size: 1, signed: false, min: 0, max: 1, step: 1, default: 0, menu: &["Apagado", "Encendido"] },
    XuDef { key: "xu_ae_metering", label: "Medición de exposición", group: Group::Exposure, unit: XU1, selector: 2, size: 1, signed: false, min: 0, max: 2, step: 1, default: 0, menu: &["Promedio", "Centro ponderado", "Puntual"] },
    XuDef { key: "xu_binning", label: "Modo poca luz", group: Group::Exposure, unit: XU2, selector: 29, size: 2, signed: false, min: 0, max: 63, step: 1, default: 0, menu: &[] },
    XuDef { key: "xu_af_metering", label: "Medición de enfoque", group: Group::Focus, unit: XU1, selector: 25, size: 1, signed: false, min: 0, max: 2, step: 1, default: 0, menu: &["Promedio", "Centro", "Puntual"] },
    XuDef { key: "xu_flip", label: "Voltear / espejo", group: Group::Frame, unit: XU1, selector: 20, size: 1, signed: false, min: 0, max: 3, step: 1, default: 0, menu: &["Normal", "Espejo", "Volteado", "Espejo + volteado"] },
    XuDef { key: "xu_nr", label: "Reducción de ruido", group: Group::Detail, unit: XU1, selector: 22, size: 1, signed: false, min: 0, max: 3, step: 3, default: 2, menu: &[] },
    XuDef { key: "xu_nr2d", label: "Reducción de ruido 2D", group: Group::Detail, unit: XU2, selector: 3, size: 1, signed: false, min: 0, max: 100, step: 1, default: 0, menu: &[] },
    XuDef { key: "xu_nr3d", label: "Reducción de ruido 3D", group: Group::Detail, unit: XU2, selector: 20, size: 1, signed: false, min: 0, max: 100, step: 1, default: 50, menu: &[] },
    XuDef { key: "xu_plf", label: "Antiparpadeo (Elgato)", group: Group::Exposure, unit: XU2, selector: 14, size: 1, signed: false, min: 0, max: 2, step: 1, default: 2, menu: &["Desactivado", "50 Hz", "60 Hz"] },
    XuDef { key: "xu_usb_mode", label: "Modo de transferencia USB", group: Group::Advanced, unit: XU2, selector: 11, size: 1, signed: false, min: 0, max: 1, step: 1, default: 0, menu: &[] },
    // Locks (writers 0x1802662a0 AE_LOCK sel 4, 0x180266e70 AWB_LOCK sel 15, 0x180266960 AF_LOCK sel 26; 1 byte).
    XuDef { key: "xu_ae_lock", label: "Bloquear exposición", group: Group::Exposure, unit: XU1, selector: 4, size: 1, signed: false, min: 0, max: 1, step: 1, default: 0, menu: &[] },
    XuDef { key: "xu_awb_lock", label: "Bloquear balance de blancos", group: Group::WhiteBalance, unit: XU1, selector: 15, size: 1, signed: false, min: 0, max: 1, step: 1, default: 0, menu: &[] },
    XuDef { key: "xu_af_lock", label: "Bloquear enfoque", group: Group::Focus, unit: XU1, selector: 26, size: 1, signed: false, min: 0, max: 1, step: 1, default: 0, menu: &[] },
];

// ---------------------------------------------------------------- one-shot commands

/// XU1 ONE_TIME_AUTO (writer 0x180267c00): 4-byte flag word, selector 30.
pub const ONE_TIME_AUTO_SELECTOR: u8 = 30;
/// CAMERA_ONE_TIME_AUTO_* constants (SWIG getters 0x180020a6d, 0x18001d601, 0x180011432,
/// 0x180007027, 0x18001c963).
pub const ONE_TIME_AUTO_FOCUS: u32 = 1;
pub const ONE_TIME_AUTO_EXPOSURE: u32 = 2;
#[allow(dead_code)]
pub const ONE_TIME_AUTO_SHUTTER: u32 = 4;
pub const ONE_TIME_AUTO_WHITE_BALANCE: u32 = 8;
#[allow(dead_code)]
pub const ONE_TIME_AUTO_TINT: u32 = 16;

/// XU2 USER_PROFILE_SAVE (SaveUserProfileTo 0x180265a50): 1 byte to selector 7, value 1.
pub const PROFILE_SAVE_SELECTOR: u8 = 7;
/// XU2 USER_PROFILE_CLEAR (0x1802684b0): 1 byte to selector 9.
#[allow(dead_code)]
pub const PROFILE_CLEAR_SELECTOR: u8 = 9;
/// USER_PROFILE_LOAD (0x180268660): 1 byte to XU2 selector 6. ResetToFactoryDefaults
/// (0x1802cb6e0) writes 0 here (factory profile), then saves with selector 7 = 1 (0x1802cb8b0).
pub const PROFILE_LOAD_SELECTOR: u8 = 6;
pub const PROFILE_FACTORY: u8 = 0;
/// SaveSettingsToDefault (0x1802cba10 → 0x1802cb8b0) writes 1 to selector 7.
pub const PROFILE_SAVE_VALUE: u8 = 1;
/// Wait after "save" before reading the stored profile back (the camera writes its memory).
pub const SAVE_SETTLE_MS: u64 = 400;

/// XU1 CAMERA_STATUS (GetCameraState 0x180261db0): 13 bytes, selector 1.
pub const CAMERA_STATUS_SELECTOR: u8 = 1;
pub const CAMERA_STATUS_LEN: usize = 13;

/// CAMERA_STATE: byte 0 isStreaming, byte 1 hasStreamed, u16 @2 width, u16 @4 height,
/// u8 @6 videoFrameRate, u32 @8 videoCodec (SWIG getters 0x1800d5560, 0x1800d5530,
/// 0x1800d55e0, 0x1800d55c0, 0x1800d55a0, 0x1800d5580).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CameraState {
    pub streaming: bool,
    pub width: u16,
    pub height: u16,
    pub fps: u8,
    pub codec: u32,
}

pub fn parse_camera_state(b: &[u8]) -> CameraState {
    CameraState {
        streaming: b[0] != 0,
        width: u16::from_le_bytes([b[2], b[3]]),
        height: u16::from_le_bytes([b[4], b[5]]),
        fps: b[6],
        codec: u32::from_le_bytes([b[8], b[9], b[10], b[11]]),
    }
}
/// XU2 REBOOT (SendRebootCommand 0x180265cb0): 1 byte 0 to selector 16.
pub const REBOOT_SELECTOR: u8 = 16;

/// Status LED (ReadLEDBrightnessAndMode 0x1802cafe0 + 0x1802cb220): pick the LED register by
/// writing 34 bytes [4, 0,0,0,0,0,0, index, ...] to XU2 selector 13 (UVC_INDEX, 0x1802cd830),
/// then read 10 bytes from XU2 selector 4 (LED_CONTROL); byte 5 is the brightness 0..255.
/// Index 3 = while streaming, 2 = idle. Percent = raw * 100 / 255.
pub const UVC_INDEX_SELECTOR: u8 = 13;
pub const UVC_INDEX_LEN: usize = 34;
pub const LED_SELECTOR: u8 = 4;
pub const LED_LEN: usize = 10;
pub const LED_BRIGHTNESS_BYTE: usize = 5;
pub const LED_INDEX_STREAMING: u8 = 3;
pub const LED_INDEX_IDLE: u8 = 2;
pub const LED_RAW_MAX: i32 = 255;

/// CAMERA_LED_MODE: 0 always off, 1 on while streaming, 2 always on (derived in 0x1802cafe0).
/// XU2 selector 8: the camera's saved profile as JSON (documented by the open-source
/// ElgatoCam project for the Facecam 4K; a 4096-byte get/set control).
pub const PROFILE_JSON_SELECTOR: u8 = 8;

pub enum ProfileTarget {
    Std(u32),
    StdAeMode,
    Xu(&'static str),
}

/// Profile JSON keys → our controls. Standard ones carry UVC units, the same as V4L2.
pub const PROFILE_KEYS: &[(&str, &str, ProfileTarget)] = &[
    ("AE_Mode", "Exposición automática", ProfileTarget::StdAeMode),
    ("ExpoTime", "Tiempo de exposición", ProfileTarget::Std(0x009a_0902)),
    ("Gain", "Ganancia / ISO", ProfileTarget::Std(0x0098_0913)),
    ("Brightness", "Brillo", ProfileTarget::Std(CID_BRIGHTNESS)),
    ("Contrast", "Contraste", ProfileTarget::Std(0x0098_0901)),
    ("Saturation", "Saturación", ProfileTarget::Std(0x0098_0902)),
    ("Hue", "Matiz", ProfileTarget::Std(0x0098_0903)),
    ("Sharpness", "Nitidez", ProfileTarget::Std(0x0098_091b)),
    ("Gamma", "Gamma", ProfileTarget::Std(0x0098_0910)),
    ("Backlight", "Compensación de contraluz", ProfileTarget::Std(CID_BACKLIGHT)),
    ("WB_Auto", "Balance de blancos automático", ProfileTarget::Std(0x0098_090c)),
    ("WB_Temp", "Temperatura (K)", ProfileTarget::Std(0x0098_091a)),
    ("PowerFreq", "Antiparpadeo", ProfileTarget::Std(0x0098_0918)),
    ("DZoom_Ratio", "Zoom", ProfileTarget::Std(CID_ZOOM)),
    ("DPan_Ofst", "Desplazamiento (pan)", ProfileTarget::Std(CID_PAN)),
    ("DTilt_Ofst", "Inclinación (tilt)", ProfileTarget::Std(CID_TILT)),
    ("AE_Meter", "Medición de exposición", ProfileTarget::Xu("xu_ae_metering")),
    ("NR2D", "Reducción de ruido 2D", ProfileTarget::Xu("xu_nr2d")),
    ("NR3D", "Reducción de ruido 3D", ProfileTarget::Xu("xu_nr3d")),
    ("Binning_Mode", "Modo poca luz", ProfileTarget::Xu("xu_binning")),
];

/// UVC CT_AE_MODE bitmap (1 manual, 2 auto, 4 shutter priority, 8 aperture priority)
/// → V4L2 exposure_auto menu (0 auto, 1 manual, 2 shutter priority, 3 aperture priority).
pub fn uvc_ae_mode_to_v4l2(m: i32) -> Option<i32> {
    match m {
        1 => Some(1),
        2 => Some(0),
        4 => Some(2),
        8 => Some(3),
        _ => None,
    }
}

/// The profile may store the low-light option index (0..2) or the raw bitfield.
pub fn low_light_from_profile(v: i32) -> i32 {
    match v {
        1 => LOW_LIGHT_BELOW_720P,
        2 => LOW_LIGHT_BELOW_1080P,
        other => low_light_from_bitfield(other),
    }
}

/// LED write (0x1802c6a40 → 0x1802cd760): 10 bytes straight to XU2 selector 4, no index select:
/// [255, 0, index, 0, 0, raw, 0, 0, 0, 0], first for index 3 (streaming), then index 2 (idle).
/// raw = percent × 255 / 100 with percent clamped to 0..100 (0x1802cbf70); mode 0 writes 0 to
/// both, 1 writes raw while streaming and 0 idle, 2 writes raw to both (0x1802cc100 refuses ≥ 3).
pub const LED_WRITE_TAG: u8 = 255;
pub const LED_INDEX_BYTE: usize = 2;
/// Defaults of the original's LED properties (0x1802c8110: brightness 68, 0..100, default 100;
/// 0x1802c81f0: mode 72, 0..2, default 1).
#[allow(dead_code)]
pub const LED_BRIGHTNESS_DEFAULT: i32 = 100;
#[allow(dead_code)]
pub const LED_MODE_DEFAULT: i32 = 1;

pub fn led_write_payload(index: u8, raw: u8) -> [u8; LED_LEN] {
    let mut b = [0u8; LED_LEN];
    b[0] = LED_WRITE_TAG;
    b[LED_INDEX_BYTE] = index;
    b[LED_BRIGHTNESS_BYTE] = raw;
    b
}

pub fn led_mode(streaming_raw: i32, idle_raw: i32) -> i32 {
    match (streaming_raw != 0, idle_raw != 0) {
        (_, true) => 2,
        (true, false) => 1,
        _ => 0,
    }
}

/// Read-only live values (XU1), shown in the info strip.
pub const XU_READOUTS: &[(&str, u8, u8)] = &[
    ("Ganancia actual", XU1, 0x08),
    ("Tiempo de exposición actual", XU1, 0x0b),
    ("Temperatura actual (K)", XU1, 0x11),
    ("Posición de enfoque", XU1, 0x0d),
];

pub fn encode(def: &XuDef, v: i32) -> Vec<u8> {
    let b = v.to_le_bytes();
    b[..def.size as usize].to_vec()
}

pub fn decode(def: &XuDef, data: &[u8]) -> i32 {
    match (def.size, def.signed) {
        (1, false) => data[0] as i32,
        (1, true) => data[0] as i8 as i32,
        (2, false) => u16::from_le_bytes([data[0], data[1]]) as i32,
        (2, true) => i16::from_le_bytes([data[0], data[1]]) as i32,
        _ => i32::from_le_bytes([data[0], data[1], data[2], data[3]]),
    }
}

/// Spanish label, group and display order for standard V4L2 controls.
pub fn std_control_meta(id: u32, name: &str) -> (String, Group, i32) {
    let (label, group, order): (&str, Group, i32) = match id {
        0x009a_090d => ("Zoom", Group::Frame, 0),
        0x009a_0908 => ("Desplazamiento horizontal (pan)", Group::Frame, 1),
        0x009a_0909 => ("Inclinación (tilt)", Group::Frame, 2),
        0x009a_0901 => ("Exposición automática", Group::Exposure, 0),
        0x009a_0903 => ("Prioridad de exposición (permite bajar fps)", Group::Exposure, 1),
        0x009a_0902 => ("Velocidad de obturación (exposición)", Group::Exposure, 2),
        0x0098_0913 => ("Ganancia / ISO", Group::Exposure, 3),
        0x0098_0918 => ("Antiparpadeo", Group::Exposure, 5),
        0x0098_091c => ("Compensación de contraluz", Group::Exposure, 6),
        0x009a_090c => ("Enfoque automático", Group::Focus, 0),
        0x009a_090a => ("Distancia de enfoque", Group::Focus, 1),
        0x0098_090c => ("Balance de blancos automático", Group::WhiteBalance, 0),
        0x0098_091a => ("Temperatura (K)", Group::WhiteBalance, 1),
        0x0098_0900 => ("Brillo", Group::Picture, 0),
        0x0098_0901 => ("Contraste", Group::Picture, 1),
        0x0098_0902 => ("Saturación", Group::Picture, 2),
        0x0098_0903 => ("Matiz", Group::Picture, 3),
        0x0098_0910 => ("Gamma", Group::Picture, 4),
        0x0098_091b => ("Nitidez", Group::Detail, 0),
        _ => ("", Group::Advanced, 50),
    };
    let label = if label.is_empty() { name.to_string() } else { label.to_string() };
    (label, group, order)
}

/// Standard menu entries translated.
pub fn translate_menu(s: &str) -> String {
    match s {
        "Disabled" => "Desactivado".into(),
        "50 Hz" => "50 Hz".into(),
        "60 Hz" => "60 Hz".into(),
        "Auto" => "Automático".into(),
        "Auto Mode" => "Automática".into(),
        "Manual Mode" => "Manual".into(),
        "Shutter Priority Mode" => "Prioridad de obturación".into(),
        "Aperture Priority Mode" => "Automática (prioridad de apertura)".into(),
        other => other.into(),
    }
}

// ---------------------------------------------------------------- rules read from the original

/// Low light mode (LowLightSettingViewModel::IndexToBitfield, Elgato.Studio.dll):
/// option 0 "Off" → 0, option 1 "Below 720p" → 60, option 2 "Below 1080p" → 62.
pub const LOW_LIGHT_OFF: i32 = 0;
pub const LOW_LIGHT_BELOW_720P: i32 = 60;
pub const LOW_LIGHT_BELOW_1080P: i32 = 62;

/// LowLightSettingViewModel::BitfieldToIndex, expressed as the bitfield to show.
pub fn low_light_from_bitfield(bits: i32) -> i32 {
    match bits {
        0 => LOW_LIGHT_OFF,
        LOW_LIGHT_BELOW_720P => LOW_LIGHT_BELOW_720P,
        LOW_LIGHT_BELOW_1080P => LOW_LIGHT_BELOW_1080P,
        b if b & 2 == 0 => LOW_LIGHT_BELOW_720P,
        _ => LOW_LIGHT_BELOW_1080P,
    }
}

/// HDR is refused when streaming faster than this (FUN_1802cf390: 0x1e, or 0x3c on one model).
pub const HDR_MAX_FPS: f32 = 30.0;
pub const HDR_MAX_FPS_EXTENDED: f32 = 60.0;

/// CAMERA_FRAME_PRESET_COUNT (0x180016f2c) and CAMERA_FRAME_PRESET_NAME_MAX_LEN (0x18000d797).
pub const FRAME_PRESET_COUNT: usize = 4;
pub const FRAME_PRESET_NAME_MAX_LEN: usize = 64;

/// Dynamic range (SetVirtualDynamicRangeValue 0x1802cf4a0): HDR on only for "High" (2),
/// backlight compensation on only for "Wide" (1) (thunk 0x1802cefc0: `mode == 1`).
pub fn dynamic_range_writes(mode: i32) -> (i32, i32) {
    (if mode == 2 { 1 } else { 0 }, if mode == 1 { 1 } else { 0 })
}

/// Reading it back (FUN_1802cefd0): HDR on → 2; else BLC on (if Wide is offered) → 1; else 0.
pub fn dynamic_range_read(hdr: i32, blc: i32, wide_supported: bool) -> i32 {
    if hdr == 1 {
        2
    } else if wide_supported && blc != 0 {
        1
    } else {
        0
    }
}

/// Original CAMERA_UVC_PROPERTY_ID numbers the virtual properties write to, mapped to V4L2:
/// 20 = PU_BRIGHTNESS (EV compensation target, 0x1802ccd70), 29 = PU_BACKLIGHT_COMPENSATION
/// (dynamic range "Wide", 0x1802cf4a0).
pub const PROP_BRIGHTNESS: i32 = 20;
pub const PROP_BACKLIGHT_COMPENSATION: i32 = 29;
pub fn original_prop_to_v4l2(prop: i32) -> Option<u32> {
    match prop {
        PROP_BRIGHTNESS => Some(CID_BRIGHTNESS),
        PROP_BACKLIGHT_COMPENSATION => Some(CID_BACKLIGHT),
        _ => None,
    }
}

pub const CID_EXPOSURE_AUTO: u32 = 0x009a_0901;

/// Original property ids (CAMERA_UVC_PROPERTY_ID, name table at 0x180643120).
pub const PROP_CT_EXPOSURE_TIME: i32 = 2;
pub const PROP_CT_FOCUS: i32 = 4;
pub const PROP_CT_ZOOM: i32 = 7;
pub const PROP_CT_PAN: i32 = 8;
pub const PROP_CT_TILT: i32 = 10;
pub const PROP_PU_WHITE_BALANCE_TEMP: i32 = 27;
pub const PROP_XU_WHITE_BALANCE_TINT: i32 = 50;
pub const PROP_XU_EXPOSURE_TIME_ABSOLUTE: i32 = 53;
pub const CID_WB_TEMP: u32 = 0x0098_091a;

/// Throttler table built at start-up (0x180055a40): writes of these properties are spaced at
/// least 50 ms apart; the last value always lands (Throttler::ApplyPendingAction 0x180273890,
/// timing loop 0x180274500). SetUVCValue (0x180293170) looks the interval up per property.
pub const THROTTLE_MS: u64 = 50;
pub const THROTTLED_PROPS: [i32; 6] = [PROP_CT_FOCUS, PROP_CT_ZOOM, PROP_CT_PAN, PROP_CT_TILT, PROP_PU_WHITE_BALANCE_TEMP, PROP_XU_WHITE_BALANCE_TINT];

/// Properties whose value the camera drives while an automatic mode is on (table 0x1806624a0,
/// released by 0x1802c31d0 before saving): they aren't compared after a save while automatic.
#[allow(dead_code)]
pub const AUTO_DRIVEN_PROPS: [i32; 4] = [PROP_CT_EXPOSURE_TIME, PROP_XU_EXPOSURE_TIME_ABSOLUTE, PROP_PU_WHITE_BALANCE_TEMP, PROP_CT_FOCUS];
pub const CID_EXPOSURE_ABS: u32 = 0x009a_0902;

/// Frame-preset transitions, as CameraFramePresetController does them:
/// - the visible crop of the sensor for a pan/tilt/zoom (0x1802a0260): zoom ratio = zoom / 100
///   (constant 0x180646d80), width = 3840 / ratio, height = 2160 / ratio (0x18064efc8,
///   0x18064efb8), x = (3840 − w) / 2 × (1 + pan), y = (2160 − h) / 2 × (1 − tilt), with pan and
///   tilt from −1 to 1;
/// - distance between two crops (0x1802a00c0): the largest of the top-left corner, bottom-right
///   corner and centre distances, in sensor pixels;
/// - duration (TransitionHelper::GetDuration 0x1802a0000) = distance × 0.5 (0x180646d40) / speed,
///   in ms; the controller's constructor (0x1802a2e60) sets speed = 1 and step = 30 ms;
/// - DoTransition (0x1802a3c00): steps = max(1, duration / 30), one every 30 ms; zoom moves so
///   1/zoom is linear (0x1802a0440), so the crop's edges move in straight lines; the last step
///   lands exactly on the preset.
pub const SENSOR_WIDTH: f64 = 3840.0;
pub const SENSOR_HEIGHT: f64 = 2160.0;
pub const ZOOM_UNIT: f64 = 100.0;
pub const TRANSITION_DISTANCE_FACTOR: f64 = 0.5;
pub const TRANSITION_SPEED: f64 = 1.0;
pub const TRANSITION_STEP_MS: u64 = 30;

/// Visible crop [x, y, w, h] in sensor pixels; pan/tilt in −1..1, zoom ratio ≥ 1.
pub fn crop_rect(pan: f64, tilt: f64, zoom_ratio: f64) -> [f64; 4] {
    let z = zoom_ratio.max(1.0);
    let w = SENSOR_WIDTH / z;
    let h = SENSOR_HEIGHT / z;
    let x = (SENSOR_WIDTH - w) * 0.5 * (1.0 + pan);
    let y = (SENSOR_HEIGHT - h) * 0.5 * (1.0 - tilt);
    [x, y, w, h]
}

/// Pan/tilt (−1..1) that put a crop of this size at this place (inverse of crop_rect).
pub fn crop_to_pan_tilt(r: [f64; 4]) -> (f64, f64) {
    let free_x = SENSOR_WIDTH - r[2];
    let free_y = SENSOR_HEIGHT - r[3];
    let pan = if free_x > 0.5 { r[0] / (free_x * 0.5) - 1.0 } else { 0.0 };
    let tilt = if free_y > 0.5 { 1.0 - r[1] / (free_y * 0.5) } else { 0.0 };
    (pan.clamp(-1.0, 1.0), tilt.clamp(-1.0, 1.0))
}

/// Distance between two crops (0x1802a00c0).
pub fn crop_distance(a: [f64; 4], b: [f64; 4]) -> f64 {
    let d = |ax: f64, ay: f64, bx: f64, by: f64| (ax - bx).powi(2) + (ay - by).powi(2);
    let tl = d(a[0], a[1], b[0], b[1]);
    let br = d(a[0] + a[2], a[1] + a[3], b[0] + b[2], b[1] + b[3]);
    let c = d(a[0] + a[2] / 2.0, a[1] + a[3] / 2.0, b[0] + b[2] / 2.0, b[1] + b[3] / 2.0);
    tl.max(br).max(c).sqrt()
}

/// Number of 30 ms steps for a move of this distance.
pub fn transition_steps(distance: f64) -> u32 {
    let ms = distance * TRANSITION_DISTANCE_FACTOR / TRANSITION_SPEED;
    ((ms as u64) / TRANSITION_STEP_MS).max(1) as u32
}

/// Zoom ratio at step i of n (0x1802a0440): z0·z1·n / ((n − i)·z1 + i·z0).
pub fn zoom_at(z0: f64, z1: f64, n: u32, i: u32) -> f64 {
    let (n, i) = (n as f64, i as f64);
    let den = (n - i) * z1 + i * z0;
    if den <= 0.0 { z1 } else { z0 * z1 * n / den }
}
/// Focus presets (CameraFocusPresetController): SaveFocusPreset (0x1802ac5e0) refuses an index
/// of 3 or more, so there are 3; it stores the current CT_FOCUS (property 4) value, and
/// SetFocusPreset (0x1802ac850) writes it back and remembers which preset is active.
pub const FOCUS_PRESET_COUNT: usize = 3;
/// V4L2 exposure_dynamic_framerate (UVC CT_AE_PRIORITY): 1 lets the camera drop fps in low light.
pub const CID_EXPOSURE_PRIORITY: u32 = 0x009a_0903;
pub const CID_BACKLIGHT: u32 = 0x0098_091c;
pub const CID_BRIGHTNESS: u32 = 0x0098_0900;
pub const CID_ZOOM: u32 = 0x009a_090d;
pub const CID_PAN: u32 = 0x009a_0908;
pub const CID_TILT: u32 = 0x009a_0909;
#[allow(dead_code)]
pub const CID_FOCUS_AUTO: u32 = 0x009a_090c;
#[allow(dead_code)]
pub const CID_FOCUS_ABS: u32 = 0x009a_090a;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transition_math() {
        // 2× zoom, pan from far left to far right: the crop (1920 wide) slides 1920 px.
        let a = crop_rect(-1.0, 0.0, 2.0);
        let b = crop_rect(1.0, 0.0, 2.0);
        assert_eq!(a, [0.0, 540.0, 1920.0, 1080.0]);
        assert!((crop_distance(a, b) - 1920.0).abs() < 1e-9);
        // 1920 × 0.5 / 1 = 960 ms → 32 steps of 30 ms.
        assert_eq!(transition_steps(1920.0), 32);
        assert_eq!(transition_steps(0.0), 1);
        // Inverse, and zoom ends exactly where it should.
        let (p, t) = crop_to_pan_tilt(crop_rect(0.3, -0.4, 1.5));
        assert!((p - 0.3).abs() < 1e-9 && (t + 0.4).abs() < 1e-9);
        assert!((zoom_at(1.0, 4.0, 10, 10) - 4.0).abs() < 1e-9);
        assert!((zoom_at(1.0, 4.0, 10, 0) - 1.0).abs() < 1e-9);
        // 1/zoom moves in a straight line: halfway between 1× and 4× is 1.6×.
        assert!((zoom_at(1.0, 4.0, 2, 1) - 1.6).abs() < 1e-9);
    }

    #[test]
    fn led_payload() {
        // 100 % while streaming: ff 00 03 00 00 ff 00 00 00 00 (0x1802c6a40).
        assert_eq!(led_write_payload(LED_INDEX_STREAMING, 255), [255, 0, 3, 0, 0, 255, 0, 0, 0, 0]);
        assert_eq!(led_write_payload(LED_INDEX_IDLE, 0), [255, 0, 2, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(50 * LED_RAW_MAX / 100, 127);
    }
}
