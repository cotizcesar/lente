//! Minimal Video4Linux2 + UVC extension-unit access through raw ioctls.
#![allow(dead_code)]

use std::ffi::CString;
use std::io;
use std::os::raw::{c_int, c_ulong, c_void};

const fn iowr(ty: u8, nr: u8, size: usize) -> c_ulong {
    ((3u64 << 30) | ((size as u64) << 16) | ((ty as u64) << 8) | nr as u64) as c_ulong
}
const fn ior(ty: u8, nr: u8, size: usize) -> c_ulong {
    ((2u64 << 30) | ((size as u64) << 16) | ((ty as u64) << 8) | nr as u64) as c_ulong
}
const fn iow(ty: u8, nr: u8, size: usize) -> c_ulong {
    ((1u64 << 30) | ((size as u64) << 16) | ((ty as u64) << 8) | nr as u64) as c_ulong
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Capability {
    pub driver: [u8; 16],
    pub card: [u8; 32],
    pub bus_info: [u8; 32],
    pub version: u32,
    pub capabilities: u32,
    pub device_caps: u32,
    pub reserved: [u32; 3],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct QueryCtrl {
    pub id: u32,
    pub ty: u32,
    pub name: [u8; 32],
    pub minimum: i32,
    pub maximum: i32,
    pub step: i32,
    pub default_value: i32,
    pub flags: u32,
    pub reserved: [u32; 2],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Control {
    id: u32,
    value: i32,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct QueryMenu {
    id: u32,
    index: u32,
    name: [u8; 32],
    reserved: u32,
}

#[repr(C)]
struct XuQuery {
    unit: u8,
    selector: u8,
    query: u8,
    size: u16,
    data: *mut u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct PixFormat {
    pub width: u32,
    pub height: u32,
    pub pixelformat: u32,
    pub field: u32,
    pub bytesperline: u32,
    pub sizeimage: u32,
    pub colorspace: u32,
    pub priv_: u32,
    pub flags: u32,
    pub ycbcr_enc: u32,
    pub quantization: u32,
    pub xfer_func: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Format {
    ty: u32,
    _pad: u32,
    fmt: [u8; 200],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct FmtDesc {
    index: u32,
    ty: u32,
    flags: u32,
    description: [u8; 32],
    pixelformat: u32,
    mbus_code: u32,
    reserved: [u32; 3],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct FrmSizeEnum {
    index: u32,
    pixel_format: u32,
    ty: u32,
    data: [u32; 6],
    reserved: [u32; 2],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct FrmIvalEnum {
    index: u32,
    pixel_format: u32,
    width: u32,
    height: u32,
    ty: u32,
    data: [u32; 6],
    reserved: [u32; 2],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct StreamParm {
    ty: u32,
    // v4l2_captureparm: capability, capturemode, timeperframe{num,den}, extendedmode, readbuffers, reserved[4]
    parm: [u32; 50],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct RequestBuffers {
    count: u32,
    ty: u32,
    memory: u32,
    capabilities: u32,
    flags: u8,
    reserved: [u8; 3],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Buffer {
    index: u32,
    ty: u32,
    bytesused: u32,
    flags: u32,
    field: u32,
    _pad: u32,
    timestamp: [i64; 2],
    timecode: [u32; 4],
    sequence: u32,
    memory: u32,
    m: u64,
    length: u32,
    reserved2: u32,
    request_fd: i32,
    _pad2: u32,
}

const VIDIOC_QUERYCAP: c_ulong = ior(b'V', 0, std::mem::size_of::<Capability>());
const VIDIOC_ENUM_FMT: c_ulong = iowr(b'V', 2, std::mem::size_of::<FmtDesc>());
const VIDIOC_G_FMT: c_ulong = iowr(b'V', 4, std::mem::size_of::<Format>());
const VIDIOC_S_FMT: c_ulong = iowr(b'V', 5, std::mem::size_of::<Format>());
const VIDIOC_REQBUFS: c_ulong = iowr(b'V', 8, std::mem::size_of::<RequestBuffers>());
const VIDIOC_QUERYBUF: c_ulong = iowr(b'V', 9, std::mem::size_of::<Buffer>());
const VIDIOC_QBUF: c_ulong = iowr(b'V', 15, std::mem::size_of::<Buffer>());
const VIDIOC_DQBUF: c_ulong = iowr(b'V', 17, std::mem::size_of::<Buffer>());
const VIDIOC_STREAMON: c_ulong = iow(b'V', 18, 4);
const VIDIOC_STREAMOFF: c_ulong = iow(b'V', 19, 4);
const VIDIOC_G_CTRL: c_ulong = iowr(b'V', 27, std::mem::size_of::<Control>());
const VIDIOC_S_CTRL: c_ulong = iowr(b'V', 28, std::mem::size_of::<Control>());
const VIDIOC_QUERYCTRL: c_ulong = iowr(b'V', 36, std::mem::size_of::<QueryCtrl>());
const VIDIOC_QUERYMENU: c_ulong = iowr(b'V', 37, std::mem::size_of::<QueryMenu>());
const VIDIOC_ENUM_FRAMESIZES: c_ulong = iowr(b'V', 74, std::mem::size_of::<FrmSizeEnum>());
const VIDIOC_ENUM_FRAMEINTERVALS: c_ulong = iowr(b'V', 75, std::mem::size_of::<FrmIvalEnum>());
const VIDIOC_G_PARM: c_ulong = iowr(b'V', 21, std::mem::size_of::<StreamParm>());
const VIDIOC_S_PARM: c_ulong = iowr(b'V', 22, std::mem::size_of::<StreamParm>());
const UVCIOC_CTRL_QUERY: c_ulong = iowr(b'u', 0x21, std::mem::size_of::<XuQuery>());

pub const CAP_VIDEO_CAPTURE: u32 = 0x1;
pub const CAP_DEVICE_CAPS: u32 = 0x8000_0000;
const BUF_TYPE_VIDEO_CAPTURE: u32 = 1;
const MEMORY_MMAP: u32 = 1;
const CTRL_FLAG_NEXT_CTRL: u32 = 0x8000_0000;

pub const CTRL_FLAG_DISABLED: u32 = 0x1;
pub const CTRL_FLAG_READ_ONLY: u32 = 0x4;
pub const CTRL_FLAG_INACTIVE: u32 = 0x10;

pub const CTRL_TYPE_INTEGER: u32 = 1;
pub const CTRL_TYPE_BOOLEAN: u32 = 2;
pub const CTRL_TYPE_MENU: u32 = 3;
pub const CTRL_TYPE_BUTTON: u32 = 4;
pub const CTRL_TYPE_CLASS: u32 = 6;
pub const CTRL_TYPE_INTEGER_MENU: u32 = 9;

pub const UVC_SET_CUR: u8 = 0x01;
pub const UVC_GET_CUR: u8 = 0x81;
pub const UVC_GET_MIN: u8 = 0x82;
pub const UVC_GET_MAX: u8 = 0x83;
pub const UVC_GET_DEF: u8 = 0x87;
pub const UVC_GET_LEN: u8 = 0x85;
pub const UVC_GET_INFO: u8 = 0x86;

pub const fn fourcc(s: &[u8; 4]) -> u32 {
    (s[0] as u32) | ((s[1] as u32) << 8) | ((s[2] as u32) << 16) | ((s[3] as u32) << 24)
}
pub const FMT_MJPG: u32 = fourcc(b"MJPG");
pub const FMT_YUYV: u32 = fourcc(b"YUYV");
pub const FMT_NV12: u32 = fourcc(b"NV12");

pub fn fourcc_str(f: u32) -> String {
    let b = f.to_le_bytes();
    b.iter().map(|&c| if c.is_ascii_graphic() { c as char } else { '?' }).collect()
}

pub fn cstr(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).trim().to_string()
}

fn xioctl(fd: c_int, req: c_ulong, arg: *mut c_void) -> io::Result<()> {
    loop {
        let r = unsafe { libc::ioctl(fd, req as _, arg) };
        if r == -1 {
            let e = io::Error::last_os_error();
            if e.raw_os_error() == Some(libc::EINTR) {
                continue;
            }
            return Err(e);
        }
        return Ok(());
    }
}

/// An open /dev/videoN.
pub struct Device {
    fd: c_int,
}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe {
            libc::close(self.fd);
        }
    }
}

#[derive(Clone, Debug)]
pub struct CtrlInfo {
    pub id: u32,
    pub ty: u32,
    pub name: String,
    pub min: i32,
    pub max: i32,
    pub step: i32,
    pub default: i32,
    pub flags: u32,
    pub menu: Vec<(i32, String)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormatInfo {
    pub fourcc: u32,
    pub description: String,
    pub sizes: Vec<(u32, u32)>,
    /// Frame intervals (numerator, denominator) for each entry of `sizes`, fastest first.
    pub intervals: Vec<Vec<(u32, u32)>>,
}

impl FormatInfo {
    pub fn fps_for(&self, w: u32, h: u32) -> Vec<(u32, u32)> {
        self.sizes.iter().position(|s| *s == (w, h)).and_then(|i| self.intervals.get(i).cloned()).unwrap_or_default()
    }
}

pub fn fps_value(iv: (u32, u32)) -> f32 {
    if iv.0 == 0 {
        0.0
    } else {
        iv.1 as f32 / iv.0 as f32
    }
}

impl Device {
    pub fn open(path: &str) -> io::Result<Self> {
        let c = CString::new(path).map_err(|_| io::Error::from(io::ErrorKind::InvalidInput))?;
        let fd = unsafe { libc::open(c.as_ptr(), libc::O_RDWR | libc::O_NONBLOCK | libc::O_CLOEXEC) };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Self { fd })
    }

    pub fn fd(&self) -> c_int {
        self.fd
    }

    pub fn query_cap(&self) -> io::Result<Capability> {
        let mut cap: Capability = unsafe { std::mem::zeroed() };
        xioctl(self.fd, VIDIOC_QUERYCAP, &mut cap as *mut _ as *mut c_void)?;
        Ok(cap)
    }

    pub fn controls(&self) -> Vec<CtrlInfo> {
        let mut out = Vec::new();
        let mut id = CTRL_FLAG_NEXT_CTRL;
        loop {
            let mut q: QueryCtrl = unsafe { std::mem::zeroed() };
            q.id = id;
            if xioctl(self.fd, VIDIOC_QUERYCTRL, &mut q as *mut _ as *mut c_void).is_err() {
                break;
            }
            id = q.id | CTRL_FLAG_NEXT_CTRL;
            if q.ty == CTRL_TYPE_CLASS || q.flags & CTRL_FLAG_DISABLED != 0 {
                continue;
            }
            let mut menu = Vec::new();
            if q.ty == CTRL_TYPE_MENU || q.ty == CTRL_TYPE_INTEGER_MENU {
                for i in q.minimum..=q.maximum {
                    let mut m: QueryMenu = unsafe { std::mem::zeroed() };
                    m.id = q.id;
                    m.index = i as u32;
                    if xioctl(self.fd, VIDIOC_QUERYMENU, &mut m as *mut _ as *mut c_void).is_ok() {
                        let name = if q.ty == CTRL_TYPE_MENU {
                            let n = m.name;
                            cstr(&n)
                        } else {
                            let n = m.name;
                            let v = i64::from_le_bytes(n[..8].try_into().unwrap());
                            v.to_string()
                        };
                        menu.push((i, name));
                    }
                }
            }
            out.push(CtrlInfo {
                id: q.id,
                ty: q.ty,
                name: cstr(&q.name),
                min: q.minimum,
                max: q.maximum,
                step: q.step,
                default: q.default_value,
                flags: q.flags,
                menu,
            });
        }
        out
    }

    /// Re-read just the flags of one control (to learn when it becomes inactive).
    pub fn ctrl_flags(&self, id: u32) -> Option<u32> {
        let mut q: QueryCtrl = unsafe { std::mem::zeroed() };
        q.id = id;
        xioctl(self.fd, VIDIOC_QUERYCTRL, &mut q as *mut _ as *mut c_void).ok()?;
        Some(q.flags)
    }

    pub fn get_ctrl(&self, id: u32) -> io::Result<i32> {
        let mut c = Control { id, value: 0 };
        xioctl(self.fd, VIDIOC_G_CTRL, &mut c as *mut _ as *mut c_void)?;
        Ok(c.value)
    }

    pub fn set_ctrl(&self, id: u32, value: i32) -> io::Result<()> {
        let mut c = Control { id, value };
        xioctl(self.fd, VIDIOC_S_CTRL, &mut c as *mut _ as *mut c_void)
    }

    pub fn xu_query(&self, unit: u8, selector: u8, query: u8, data: &mut [u8]) -> io::Result<()> {
        let mut q = XuQuery { unit, selector, query, size: data.len() as u16, data: data.as_mut_ptr() };
        xioctl(self.fd, UVCIOC_CTRL_QUERY, &mut q as *mut _ as *mut c_void)
    }

    pub fn formats(&self) -> Vec<FormatInfo> {
        let mut out = Vec::new();
        for index in 0..32 {
            let mut d: FmtDesc = unsafe { std::mem::zeroed() };
            d.index = index;
            d.ty = BUF_TYPE_VIDEO_CAPTURE;
            if xioctl(self.fd, VIDIOC_ENUM_FMT, &mut d as *mut _ as *mut c_void).is_err() {
                break;
            }
            let mut sizes = Vec::new();
            for si in 0..64 {
                let mut s: FrmSizeEnum = unsafe { std::mem::zeroed() };
                s.index = si;
                s.pixel_format = d.pixelformat;
                if xioctl(self.fd, VIDIOC_ENUM_FRAMESIZES, &mut s as *mut _ as *mut c_void).is_err() {
                    break;
                }
                if s.ty == 1 {
                    sizes.push((s.data[0], s.data[1]));
                } else {
                    // stepwise / continuous: offer the max size
                    sizes.push((s.data[1], s.data[4]));
                    break;
                }
            }
            sizes.sort_by(|a, b| (b.0 * b.1).cmp(&(a.0 * a.1)));
            sizes.dedup();
            let intervals = sizes.iter().map(|&(w, h)| self.frame_intervals(d.pixelformat, w, h)).collect();
            let desc = d.description;
            out.push(FormatInfo { fourcc: d.pixelformat, description: cstr(&desc), sizes, intervals });
        }
        out
    }

    pub fn frame_intervals(&self, fourcc: u32, w: u32, h: u32) -> Vec<(u32, u32)> {
        let mut out = Vec::new();
        for index in 0..32 {
            let mut f: FrmIvalEnum = unsafe { std::mem::zeroed() };
            f.index = index;
            f.pixel_format = fourcc;
            f.width = w;
            f.height = h;
            if xioctl(self.fd, VIDIOC_ENUM_FRAMEINTERVALS, &mut f as *mut _ as *mut c_void).is_err() {
                break;
            }
            if f.ty == 1 {
                out.push((f.data[0], f.data[1]));
            } else {
                // stepwise/continuous: offer min and max interval
                out.push((f.data[0], f.data[1]));
                out.push((f.data[2], f.data[3]));
                break;
            }
        }
        out.sort_by(|a, b| fps_value(*b).partial_cmp(&fps_value(*a)).unwrap_or(std::cmp::Ordering::Equal));
        out.dedup();
        out
    }

    /// Ask for a frame interval (VIDIOC_S_PARM); returns what the driver chose.
    pub fn set_frame_interval(&self, num: u32, den: u32) -> io::Result<(u32, u32)> {
        let mut p: StreamParm = unsafe { std::mem::zeroed() };
        p.ty = BUF_TYPE_VIDEO_CAPTURE;
        p.parm[2] = num;
        p.parm[3] = den;
        xioctl(self.fd, VIDIOC_S_PARM, &mut p as *mut _ as *mut c_void)?;
        Ok((p.parm[2], p.parm[3]))
    }

    #[allow(dead_code)]
    pub fn frame_interval(&self) -> io::Result<(u32, u32)> {
        let mut p: StreamParm = unsafe { std::mem::zeroed() };
        p.ty = BUF_TYPE_VIDEO_CAPTURE;
        xioctl(self.fd, VIDIOC_G_PARM, &mut p as *mut _ as *mut c_void)?;
        Ok((p.parm[2], p.parm[3]))
    }

    pub fn set_format(&self, fourcc: u32, w: u32, h: u32) -> io::Result<PixFormat> {
        let mut f: Format = unsafe { std::mem::zeroed() };
        f.ty = BUF_TYPE_VIDEO_CAPTURE;
        let pix = PixFormat {
            width: w,
            height: h,
            pixelformat: fourcc,
            field: 1, // V4L2_FIELD_NONE
            bytesperline: 0,
            sizeimage: 0,
            colorspace: 0,
            priv_: 0,
            flags: 0,
            ycbcr_enc: 0,
            quantization: 0,
            xfer_func: 0,
        };
        unsafe {
            std::ptr::copy_nonoverlapping(
                &pix as *const _ as *const u8,
                f.fmt.as_mut_ptr(),
                std::mem::size_of::<PixFormat>(),
            );
        }
        xioctl(self.fd, VIDIOC_S_FMT, &mut f as *mut _ as *mut c_void)?;
        let mut got: PixFormat = unsafe { std::mem::zeroed() };
        unsafe {
            std::ptr::copy_nonoverlapping(
                f.fmt.as_ptr(),
                &mut got as *mut _ as *mut u8,
                std::mem::size_of::<PixFormat>(),
            );
        }
        Ok(got)
    }
}

/// Memory-mapped capture stream.
pub struct Stream<'a> {
    dev: &'a Device,
    bufs: Vec<(*mut c_void, usize)>,
}

impl<'a> Stream<'a> {
    pub fn start(dev: &'a Device, count: u32) -> io::Result<Self> {
        let mut req = RequestBuffers {
            count,
            ty: BUF_TYPE_VIDEO_CAPTURE,
            memory: MEMORY_MMAP,
            capabilities: 0,
            flags: 0,
            reserved: [0; 3],
        };
        xioctl(dev.fd, VIDIOC_REQBUFS, &mut req as *mut _ as *mut c_void)?;
        let mut bufs = Vec::new();
        for i in 0..req.count {
            let mut b: Buffer = unsafe { std::mem::zeroed() };
            b.index = i;
            b.ty = BUF_TYPE_VIDEO_CAPTURE;
            b.memory = MEMORY_MMAP;
            xioctl(dev.fd, VIDIOC_QUERYBUF, &mut b as *mut _ as *mut c_void)?;
            let ptr = unsafe {
                libc::mmap(
                    std::ptr::null_mut(),
                    b.length as usize,
                    libc::PROT_READ | libc::PROT_WRITE,
                    libc::MAP_SHARED,
                    dev.fd,
                    (b.m & 0xffff_ffff) as libc::off_t,
                )
            };
            if ptr == libc::MAP_FAILED {
                return Err(io::Error::last_os_error());
            }
            bufs.push((ptr, b.length as usize));
            xioctl(dev.fd, VIDIOC_QBUF, &mut b as *mut _ as *mut c_void)?;
        }
        let mut ty: c_int = BUF_TYPE_VIDEO_CAPTURE as c_int;
        xioctl(dev.fd, VIDIOC_STREAMON, &mut ty as *mut _ as *mut c_void)?;
        Ok(Self { dev, bufs })
    }

    /// Wait up to `timeout_ms` for a frame and hand its bytes to `f`.
    pub fn next_frame<R>(&mut self, timeout_ms: i32, f: impl FnOnce(&[u8]) -> R) -> io::Result<Option<R>> {
        let mut pfd = libc::pollfd { fd: self.dev.fd, events: libc::POLLIN, revents: 0 };
        let r = unsafe { libc::poll(&mut pfd, 1, timeout_ms) };
        if r < 0 {
            return Err(io::Error::last_os_error());
        }
        if r == 0 {
            return Ok(None);
        }
        let mut b: Buffer = unsafe { std::mem::zeroed() };
        b.ty = BUF_TYPE_VIDEO_CAPTURE;
        b.memory = MEMORY_MMAP;
        match xioctl(self.dev.fd, VIDIOC_DQBUF, &mut b as *mut _ as *mut c_void) {
            Ok(()) => {}
            Err(e) if e.raw_os_error() == Some(libc::EAGAIN) => return Ok(None),
            Err(e) => return Err(e),
        }
        let (ptr, len) = self.bufs[b.index as usize];
        let used = (b.bytesused as usize).min(len);
        let data = unsafe { std::slice::from_raw_parts(ptr as *const u8, used) };
        let out = f(data);
        xioctl(self.dev.fd, VIDIOC_QBUF, &mut b as *mut _ as *mut c_void)?;
        Ok(Some(out))
    }
}

impl Drop for Stream<'_> {
    fn drop(&mut self) {
        let mut ty: c_int = BUF_TYPE_VIDEO_CAPTURE as c_int;
        let _ = xioctl(self.dev.fd, VIDIOC_STREAMOFF, &mut ty as *mut _ as *mut c_void);
        for (p, l) in &self.bufs {
            unsafe {
                libc::munmap(*p, *l);
            }
        }
        let mut req = RequestBuffers {
            count: 0,
            ty: BUF_TYPE_VIDEO_CAPTURE,
            memory: MEMORY_MMAP,
            capabilities: 0,
            flags: 0,
            reserved: [0; 3],
        };
        let _ = xioctl(self.dev.fd, VIDIOC_REQBUFS, &mut req as *mut _ as *mut c_void);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ioctl_numbers() {
        assert_eq!(VIDIOC_QUERYCAP, 0x8068_5600);
        assert_eq!(VIDIOC_QUERYCTRL, 0xC044_5624);
        assert_eq!(VIDIOC_QUERYMENU, 0xC02C_5625);
        assert_eq!(VIDIOC_G_CTRL, 0xC008_561B);
        assert_eq!(VIDIOC_S_FMT, 0xC0D0_5605);
        assert_eq!(VIDIOC_REQBUFS, 0xC014_5608);
        assert_eq!(VIDIOC_QBUF, 0xC058_560F);
        assert_eq!(VIDIOC_DQBUF, 0xC058_5611);
        assert_eq!(VIDIOC_ENUM_FMT, 0xC040_5602);
        assert_eq!(VIDIOC_ENUM_FRAMESIZES, 0xC02C_564A);
        assert_eq!(UVCIOC_CTRL_QUERY, 0xC010_7521);
        assert_eq!(VIDIOC_STREAMON, 0x4004_5612);
        assert_eq!(VIDIOC_ENUM_FRAMEINTERVALS, 0xC034_564B);
        assert_eq!(VIDIOC_S_PARM, 0xC0CC_5616);
    }
}
