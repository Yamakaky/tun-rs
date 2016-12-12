use std::io;
use std::fs;
use std::net::*;
use std::os::unix::io::AsRawFd;
use std::os::unix::io::{FromRawFd, IntoRawFd};

use futures::Async;
use libc;
use tokio_core::reactor::{PollEvented, Handle};
use tokio_core::io::Io;
use mio_wrapper;

use ffi;

error_chain! {
    errors {
        NameTooLong(len: usize) {
            description("Interface name too long")
            display("Interface name too long ({} >= 16)", len)
        }
        Open {
            description("Error while opening the device")
        }
        Create {
            description("Error while creating the device")
        }
        AddIp {
            description("Error while adding an IP to the interface")
        }
    }
}

pub struct Tun {
    // TODO(tailhook) Why we need a name here?
    pub name: String,
    inner: PollEvented<mio_wrapper::Tun>,
}

impl Tun {
    pub fn new(name: &str, handle: &Handle) -> Result<Tun> {
        if name.as_bytes().len() >= 16 {
            Err(ErrorKind::NameTooLong(name.as_bytes().len()))?;
        }
        let tun = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/net/tun")
            .chain_err(|| ErrorKind::Open)?;
        let mut params = ffi::ifreq::<libc::c_short> {
            name: [0; 16],
            data: ffi::IFF_TUN,
        };
        for (from, to) in name.as_bytes().iter().zip(params.name.iter_mut()) {
            *to = *from as libc::c_schar;
        }
        let ret = unsafe {
            ffi::tun_create(tun.as_raw_fd(), &params as *const _ as *const libc::c_void as *const i32)
        };
        if ret < 0 {
            Err(io::Error::last_os_error()).chain_err(|| ErrorKind::Create)?;
        }
        Self::add_ip(params.name,
                     IpAddr::V4(Ipv4Addr::new(10, 9, 3, 2)),
                     IpAddr::V4(Ipv4Addr::new(255, 255, 255, 0)))?;
        Self::set_up(params.name)?;
        let mio = unsafe { mio_wrapper::Tun::from_raw_fd(tun.into_raw_fd()) };
        let inner = PollEvented::new(mio,handle).chain_err(|| ErrorKind::Create)?;
        Ok(Tun {
            name: name.into(),
            inner: inner,
        })
    }

    pub fn add_ip(name: [i8; 16], ip: IpAddr, mask: IpAddr) -> Result<()> {
        let socket = UdpSocket::bind("0.0.0.0:6555").chain_err(|| ErrorKind::AddIp)?;
        let mut param = ffi::ifreq {
            name: name,
            data: ffi::addr_to_raw(ip),
        };
        let ret = unsafe {
            ffi::add_ip(socket.as_raw_fd(), &mut param as *mut _ as *mut libc::c_void as *mut u8)
        };
        if ret < 0 {
            Err(io::Error::last_os_error()).chain_err(|| ErrorKind::AddIp)?;
        }
        let mut param = ffi::ifreq {
            name: name,
            data: ffi::addr_to_raw(mask),
        };
        let ret = unsafe {
            ffi::add_mask(socket.as_raw_fd(), &mut param as *mut _ as *mut libc::c_void as *mut u8)
        };
        if ret < 0 {
            Err(io::Error::last_os_error()).chain_err(|| ErrorKind::AddIp)?;
        }
        Ok(())
    }

    pub fn set_up(name: [i8; 16]) -> Result<()> {
        let socket = UdpSocket::bind("0.0.0.0:6555").chain_err(|| ErrorKind::AddIp)?;
        let mut param = ffi::ifreq {
            name: name,
            data: ffi::IFF_UP | ffi::IFF_RUNNING,
        };
        let ret = unsafe {
            ffi::set_flags(socket.as_raw_fd(), &mut param as *mut _ as *mut libc::c_void as *mut u8)
        };
        if ret < 0 {
            Err(io::Error::last_os_error()).chain_err(|| ErrorKind::AddIp)?;
        }
        Ok(())
    }
}

impl io::Read for Tun {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }
}
impl io::Write for Tun {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

impl Io for Tun {
    fn poll_read(&mut self) -> Async<()> {
        self.inner.poll_read()
    }

    fn poll_write(&mut self) -> Async<()> {
        self.inner.poll_write()
    }
}
