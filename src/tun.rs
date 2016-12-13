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
    // Used in ifreq for ioctl.
    _name: Option<String>,
    inner: PollEvented<mio_wrapper::Tun>,
}

impl Tun {
    pub fn new<S: Into<Option<String>>>(name: S, handle: &Handle) -> Result<Tun> {
        let name = name.into();
        let tun = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/net/tun")
            .chain_err(|| ErrorKind::Open)?;

        let mut params = ffi::ifreq::<libc::c_short> {
            name: [0; 16],
            data: ffi::IFF_TUN,
        };
        if let Some(ref name) = name {
            if name.as_bytes().len() >= 16 {
                Err(ErrorKind::NameTooLong(name.as_bytes().len()))?;
            }
            for (from, to) in name.as_bytes().iter().zip(params.name.iter_mut()) {
                *to = *from as libc::c_schar;
            }
        }
        ffi::check_ret(unsafe {
            ffi::tun_create(tun.as_raw_fd(),
                            &params as *const _
                                    as *const libc::c_void
                                    as *const i32) as isize
        }).chain_err(|| ErrorKind::Create)?;

        Self::add_ip4(params.name,
                      Ipv4Addr::new(10, 9, 3, 2),
                      Ipv4Addr::new(255, 255, 255, 0))?;
        Self::add_ip6(params.name,
                      Ipv6Addr::new(10, 9, 3, 2, 0, 0, 0, 0),
                      64)?;
        Self::set_up(params.name)?;
        set_nonblock(&tun).chain_err(|| ErrorKind::Create)?;
        let mio = unsafe { mio_wrapper::Tun::from_raw_fd(tun.into_raw_fd()) };
        let inner = PollEvented::new(mio,handle).chain_err(|| ErrorKind::Create)?;
        Ok(Tun {
            _name: name,
            inner: inner,
        })
    }

    fn add_ip4(name: [i8; 16], ip: Ipv4Addr, mask: Ipv4Addr) -> Result<()> {
        let socket = unsafe { UdpSocket::from_raw_fd(
            libc::socket(libc::AF_INET, libc::SOCK_DGRAM, 0)
        ) };

        let mut param = ffi::ifreq {
            name: name,
            data: ffi::addr4_to_raw(ip),
        };
        ffi::check_ret(unsafe {
            ffi::add_ip(socket.as_raw_fd(),
                          &mut param as *mut _
                                     as *mut libc::c_void
                                     as *mut u8) as isize
        }).chain_err(|| ErrorKind::AddIp)?;

        param.data = ffi::addr4_to_raw(mask);
        ffi::check_ret(unsafe {
            ffi::add_mask(socket.as_raw_fd(),
                          &mut param as *mut _
                                     as *mut libc::c_void
                                     as *mut u8) as isize
        }).chain_err(|| ErrorKind::AddIp)?;

        Ok(())
    }

    fn add_ip6(name: [i8; 16], ip: Ipv6Addr, prefixlen: u32) -> Result<()> {
        let socket = unsafe { UdpSocket::from_raw_fd(
            libc::socket(libc::AF_INET6, libc::SOCK_DGRAM, 0)
        ) };

        let mut param = ffi::ifreq::<libc::c_int> {
            name: name,
            data: 0,
        };
        ffi::check_ret(unsafe {
            ffi::get_interface_index(socket.as_raw_fd(),
                                     &mut param as *mut _
                                                as *mut libc::c_void
                                                as *mut u8) as isize
        }).chain_err(|| ErrorKind::AddIp)?;

        let mut param = ffi::in6_ifreq {
            addr: ffi::addr6_to_raw(ip),
            prefixlen: prefixlen,
            ifindex: param.data,
        };
        ffi::check_ret(unsafe {
            ffi::add_ip(socket.as_raw_fd(),
                          &mut param as *mut _
                                     as *mut libc::c_void
                                     as *mut u8) as isize
        }).chain_err(|| ErrorKind::AddIp)?;

        Ok(())
    }

    pub fn set_up(name: [i8; 16]) -> Result<()> {
        let socket = UdpSocket::bind("0.0.0.0:6555").chain_err(|| ErrorKind::AddIp)?;

        let mut param = ffi::ifreq {
            name: name,
            data: ffi::IFF_UP | ffi::IFF_RUNNING,
        };
        ffi::check_ret(unsafe {
            ffi::set_flags(socket.as_raw_fd(),
                           &mut param as *mut _
                                      as *mut libc::c_void
                                      as *mut u8) as isize
        }).chain_err(|| ErrorKind::AddIp)?;

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

fn set_nonblock(s: &AsRawFd) -> io::Result<()> {
    ffi::check_ret(unsafe {
        libc::fcntl(s.as_raw_fd(), libc::F_SETFL, libc::O_NONBLOCK) as isize
    }).map(|_| ())
}
