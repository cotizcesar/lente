//! Direct UVC requests over usbfs (`/dev/bus/usb/BBB/DDD`).
//!
//! Linux's `uvcvideo` only lets programs use the extension-unit selectors a camera lists in its
//! descriptors. The Facecam 4K leaves several of the original app's controls out of that list
//! (tint, vibrancy, flip, noise reduction, locks, refocus, reboot). The camera still answers
//! them. This module sends the same class request straight to the device, addressed with the
//! DEVICE recipient so usbfs doesn't need to take the interface away from `uvcvideo`; it works
//! while the camera is streaming. Technique documented by the open-source facecam-tui project.
//! Needs write access to the USB node (udev rule with TAG+="uaccess", see `udev_rule`).

use std::ffi::CString;
use std::io;
use std::os::raw::{c_int, c_ulong, c_void};

#[repr(C)]
struct CtrlTransfer {
    request_type: u8,
    request: u8,
    value: u16,
    index: u16,
    length: u16,
    timeout: u32,
    data: *mut c_void,
}

const USBDEVFS_CONTROL: c_ulong =
    ((3u64 << 30) | ((std::mem::size_of::<CtrlTransfer>() as u64) << 16) | ((b'U' as u64) << 8)) as c_ulong;

/// Class request, DEVICE recipient: 0x20 host→device, 0xA0 device→host.
const REQ_OUT: u8 = 0x20;
const REQ_IN: u8 = 0xA0;

pub struct DirectUsb {
    fd: c_int,
    /// bInterfaceNumber of the VideoControl interface (0 on Elgato webcams).
    vc_interface: u8,
}

impl Drop for DirectUsb {
    fn drop(&mut self) {
        unsafe {
            libc::close(self.fd);
        }
    }
}

impl DirectUsb {
    pub fn open(node: &str, vc_interface: u8) -> io::Result<Self> {
        let c = CString::new(node).map_err(|_| io::Error::from(io::ErrorKind::InvalidInput))?;
        let fd = unsafe { libc::open(c.as_ptr(), libc::O_RDWR | libc::O_CLOEXEC) };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Self { fd, vc_interface })
    }

    /// Same meaning as UVCIOC_CTRL_QUERY: `query` is a UVC request code (0x01 SET_CUR, 0x81 GET_CUR...).
    pub fn xu_query(&self, unit: u8, selector: u8, query: u8, data: &mut [u8]) -> io::Result<()> {
        let request_type = if query & 0x80 != 0 { REQ_IN } else { REQ_OUT };
        let mut t = CtrlTransfer {
            request_type,
            request: query,
            value: (selector as u16) << 8,
            index: ((unit as u16) << 8) | self.vc_interface as u16,
            length: data.len() as u16,
            timeout: if query & 0x80 != 0 { 150 } else { WRITE_TIMEOUT_MS },
            data: data.as_mut_ptr() as *mut c_void,
        };
        let mut tries = 0;
        loop {
            let r = unsafe { libc::ioctl(self.fd, USBDEVFS_CONTROL as _, &mut t as *mut CtrlTransfer) };
            if r >= 0 {
                if (r as usize) < data.len() && query & 0x80 != 0 {
                    return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "respuesta corta"));
                }
                return Ok(());
            }
            let e = io::Error::last_os_error();
            // A camera waking from USB power saving stalls requests for ~0.5 s (EPIPE). Only
            // writes (a person pressed something) retry; probes fail fast so opening stays quick.
            if e.raw_os_error() == Some(libc::EPIPE) && query & 0x80 == 0 && tries < 2 {
                tries += 1;
                std::thread::sleep(std::time::Duration::from_millis(250));
                continue;
            }
            return Err(e);
        }
    }
}

/// udev rule giving the local user access to Elgato cameras' USB nodes and turning USB power
/// saving off (the camera rejects requests for ~0.5 s after waking).
pub fn udev_rule() -> String {
    "# Lente: acceso directo por USB a cámaras Elgato (vendor 0fd9) para el usuario de la sesión\n\
SUBSYSTEM==\"usb\", ENV{DEVTYPE}==\"usb_device\", ATTR{idVendor}==\"0fd9\", TAG+=\"uaccess\"\n\
# Sin ahorro de energía USB: tras despertar, la cámara rechaza órdenes durante medio segundo\n\
ACTION==\"add\", SUBSYSTEM==\"usb\", ATTR{idVendor}==\"0fd9\", TEST==\"power/control\", ATTR{power/control}=\"on\"\n"
        .to_string()
}

/// Writes wait as long as the kernel's UVC driver does: saving to the camera's memory is slow.
pub const WRITE_TIMEOUT_MS: u32 = 5000;

pub const RULE_PATH: &str = "/etc/udev/rules.d/70-lente-elgato.rules";

/// Shell command that installs the rule (run with pkexec or sudo).
pub fn install_command() -> String {
    format!(
        "printf '%s' '{}' > {} && udevadm control --reload && udevadm trigger --subsystem-match=usb --attr-match=idVendor=0fd9",
        udev_rule().replace('\'', "'\\''"),
        RULE_PATH
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn layout() {
        assert_eq!(std::mem::size_of::<CtrlTransfer>(), 24);
        assert_eq!(USBDEVFS_CONTROL, 0xC018_5500);
    }
}
