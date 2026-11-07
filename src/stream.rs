//! Preview pipeline in two threads, so decoding never slows the camera down:
//! * capture: owns its own handle to the camera, dequeues every frame as fast as the camera
//!   sends it, copies the bytes into a "latest frame" slot and counts the camera's real fps;
//! * decode: takes the newest raw frame (older ones are skipped), turns it into RGBA and counts
//!   the preview fps.

use crate::v4l::{self, Device, Stream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use std::time::Instant;

pub struct Frame {
    pub width: usize,
    pub height: usize,
    pub rgba: Vec<u8>,
    pub seq: u64,
}

#[derive(Default)]
pub struct Shared {
    pub frame: Option<Frame>,
    /// Frames per second shown in Lente (decoded).
    pub fps: f32,
    /// Frames per second the camera actually delivers.
    pub camera_fps: f32,
    pub status: String,
    pub error: Option<String>,
    pub actual: Option<(u32, u32, u32)>,
    pub target_fps: f32,
}

#[derive(Default)]
struct Raw {
    data: Vec<u8>,
    seq: u64,
    done: bool,
}

pub struct Preview {
    stop: Arc<AtomicBool>,
    pub shared: Arc<Mutex<Shared>>,
    raw: Arc<(Mutex<Raw>, Condvar)>,
    handles: Vec<JoinHandle<()>>,
}

impl Preview {
    pub fn start(path: String, fourcc: u32, w: u32, h: u32, interval: (u32, u32), ctx: egui::Context) -> Preview {
        let stop = Arc::new(AtomicBool::new(false));
        let shared = Arc::new(Mutex::new(Shared { status: "Iniciando…".into(), ..Default::default() }));
        let raw: Arc<(Mutex<Raw>, Condvar)> = Arc::new((Mutex::new(Raw::default()), Condvar::new()));
        let mut handles = Vec::new();
        {
            let (stop, shared, raw, ctx) = (stop.clone(), shared.clone(), raw.clone(), ctx.clone());
            handles.push(std::thread::spawn(move || {
                if let Err(e) = capture(&path, fourcc, w, h, interval, &stop, &shared, &raw) {
                    let mut g = shared.lock().unwrap();
                    g.error = Some(e);
                    g.status = "Vista previa detenida".into();
                    ctx.request_repaint();
                }
                let (m, cv) = &*raw;
                m.lock().unwrap().done = true;
                cv.notify_all();
            }));
        }
        {
            let (stop, shared, raw) = (stop.clone(), shared.clone(), raw.clone());
            handles.push(std::thread::spawn(move || decode_loop(&stop, &shared, &raw, &ctx)));
        }
        Preview { stop, shared, raw, handles }
    }
}

impl Drop for Preview {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let (m, cv) = &*self.raw;
        m.lock().unwrap().done = true;
        cv.notify_all();
        for h in self.handles.drain(..) {
            let _ = h.join();
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn capture(
    path: &str,
    fourcc: u32,
    w: u32,
    h: u32,
    interval: (u32, u32),
    stop: &AtomicBool,
    shared: &Mutex<Shared>,
    raw: &(Mutex<Raw>, Condvar),
) -> Result<(), String> {
    let dev = Device::open(path).map_err(|e| match e.raw_os_error() {
        Some(libc::EBUSY) => "La cámara está ocupada por otra aplicación".to_string(),
        _ => format!("No se pudo abrir la cámara: {e}"),
    })?;
    let got = dev.set_format(fourcc, w, h).map_err(|e| match e.raw_os_error() {
        Some(libc::EBUSY) => "La cámara está transmitiendo en otra aplicación (OBS, navegador…). Los ajustes siguen funcionando.".to_string(),
        _ => format!("Formato no aceptado: {e}"),
    })?;
    // Frame rate (VIDIOC_S_PARM); the camera may round it.
    let iv = if interval.0 > 0 { dev.set_frame_interval(interval.0, interval.1).unwrap_or(interval) } else { (0, 0) };
    {
        let mut g = shared.lock().unwrap();
        g.actual = Some((got.width, got.height, got.pixelformat));
        g.target_fps = v4l::fps_value(iv);
        g.status = format!("{}×{} {}", got.width, got.height, v4l::fourcc_str(got.pixelformat));
    }
    let mut stream = Stream::start(&dev, 4).map_err(|e| format!("No se pudo iniciar la transmisión: {e}"))?;
    let mut last = Instant::now();
    let mut count = 0u32;
    let mut seq = 0u64;
    let (m, cv) = raw;
    while !stop.load(Ordering::SeqCst) {
        let got_frame = stream
            .next_frame(200, |data| {
                let mut r = m.lock().unwrap();
                r.data.clear();
                r.data.extend_from_slice(data);
                seq += 1;
                r.seq = seq;
                cv.notify_one();
            })
            .map_err(|e| format!("Error de captura: {e}"))?;
        if got_frame.is_some() {
            count += 1;
        }
        let el = last.elapsed().as_secs_f32();
        if el >= 1.0 {
            shared.lock().unwrap().camera_fps = count as f32 / el;
            count = 0;
            last = Instant::now();
        }
    }
    Ok(())
}

fn decode_loop(stop: &AtomicBool, shared: &Mutex<Shared>, raw: &(Mutex<Raw>, Condvar), ctx: &egui::Context) {
    let (m, cv) = raw;
    let mut last_seq = 0u64;
    let mut buf = Vec::new();
    let mut last = Instant::now();
    let mut count = 0u32;
    let mut bad = 0u32;
    loop {
        {
            let mut r = m.lock().unwrap();
            while r.seq == last_seq && !r.done {
                r = cv.wait(r).unwrap();
            }
            if r.done || stop.load(Ordering::SeqCst) {
                return;
            }
            last_seq = r.seq;
            std::mem::swap(&mut buf, &mut r.data);
        }
        let (w, h, ff) = match shared.lock().unwrap().actual {
            Some((w, h, f)) => (w as usize, h as usize, f),
            None => continue,
        };
        match decode(&buf, w, h, ff) {
            Some((width, height, rgba)) => {
                bad = 0;
                count += 1;
                let el = last.elapsed().as_secs_f32();
                let mut g = shared.lock().unwrap();
                if el >= 1.0 {
                    g.fps = count as f32 / el;
                    count = 0;
                    last = Instant::now();
                }
                g.frame = Some(Frame { width, height, rgba, seq: last_seq });
                drop(g);
                ctx.request_repaint();
            }
            None => {
                bad += 1;
                if bad > 30 {
                    let mut g = shared.lock().unwrap();
                    g.error = Some("No se pudieron decodificar los fotogramas; prueba otro formato".into());
                    ctx.request_repaint();
                    return;
                }
            }
        }
    }
}

fn decode(data: &[u8], w: usize, h: usize, fourcc: u32) -> Option<(usize, usize, Vec<u8>)> {
    match fourcc {
        v4l::FMT_MJPG => {
            // zune-jpeg straight to RGBA (no extra conversion pass); handles MJPEG without
            // Huffman tables by using the standard ones.
            let opts = zune_core::options::DecoderOptions::default().jpeg_set_out_colorspace(zune_core::colorspace::ColorSpace::RGBA);
            let mut dec = zune_jpeg::JpegDecoder::new_with_options(zune_core::bytestream::ZCursor::new(data), opts);
            let px = dec.decode().ok()?;
            let info = dec.info()?;
            Some((info.width as usize, info.height as usize, px))
        }
        v4l::FMT_YUYV => {
            if data.len() < w * h * 2 {
                return None;
            }
            let m = Matrix::for_height(h);
            let mut out = vec![255u8; w * h * 4];
            for (i, px) in data[..w * h * 2].chunks_exact(4).enumerate() {
                let (u, v) = (px[1] as i32 - 128, px[3] as i32 - 128);
                m.put(&mut out[i * 8..i * 8 + 4], px[0], u, v);
                m.put(&mut out[i * 8 + 4..i * 8 + 8], px[2], u, v);
            }
            Some((w, h, out))
        }
        v4l::FMT_NV12 => {
            if data.len() < w * h * 3 / 2 {
                return None;
            }
            let m = Matrix::for_height(h);
            let (yp, uv) = data.split_at(w * h);
            let mut out = vec![255u8; w * h * 4];
            for row in 0..h {
                let uvrow = &uv[(row / 2) * w..];
                for col in 0..w {
                    let ci = (col / 2) * 2;
                    let o = (row * w + col) * 4;
                    m.put(&mut out[o..o + 4], yp[row * w + col], uvrow[ci] as i32 - 128, uvrow[ci + 1] as i32 - 128);
                }
            }
            Some((w, h, out))
        }
        _ => None,
    }
}

/// Limited-range YUV → RGB in fixed point. HD sizes use BT.709 (what Windows' camera stack
/// assumes for HD video), SD uses BT.601, so colours match the Windows app.
struct Matrix {
    rv: i32,
    gu: i32,
    gv: i32,
    bu: i32,
}

impl Matrix {
    fn for_height(h: usize) -> Matrix {
        if h >= 720 {
            Matrix { rv: 459, gu: -55, gv: -136, bu: 541 } // BT.709 ×256
        } else {
            Matrix { rv: 409, gu: -100, gv: -208, bu: 516 } // BT.601 ×256
        }
    }
    #[inline]
    fn put(&self, out: &mut [u8], y: u8, u: i32, v: i32) {
        let c = (y as i32 - 16) * 298;
        out[0] = ((c + self.rv * v + 128) >> 8).clamp(0, 255) as u8;
        out[1] = ((c + self.gu * u + self.gv * v + 128) >> 8).clamp(0, 255) as u8;
        out[2] = ((c + self.bu * u + 128) >> 8).clamp(0, 255) as u8;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn yuv_white_black() {
        let m = Matrix::for_height(1080);
        let mut px = [0u8; 4];
        m.put(&mut px, 235, 0, 0);
        assert_eq!(&px[..3], &[255, 255, 255]);
        m.put(&mut px, 16, 0, 0);
        assert_eq!(&px[..3], &[0, 0, 0]);
    }
}
