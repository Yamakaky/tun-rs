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

mod linux {
    use libc;

    ioctl!(write tun_create with b'T', 202; libc::c_int);
    ioctl!(bad add_ip with SIOCSIFADDR);
    ioctl!(bad add_mask with SIOCSIFNETMASK);
    ioctl!(bad set_flags with SIOCSIFFLAGS);

    #[repr(C)]
    pub struct ifreq<T> {
        pub name: [libc::c_char; 16],
        pub data: T,
    }
    pub const SIOCSIFADDR: libc::c_ushort = 0x8916;
    pub const SIOCSIFNETMASK: libc::c_ushort = 0x891c;
    pub const SIOCSIFFLAGS: libc::c_ushort = 0x8914;
    pub const IFF_TUN: libc::c_short = 0x0001;

    bitflags! {
        #[repr(C)]
        pub flags DevConfigFlags: libc::c_ushort {
            const IFF_UP = 0x1,		/* Interface is up.  */
            const IFF_BROADCAST = 0x2,	/* Broadcast address valid.  */
            const IFF_DEBUG = 0x4,		/* Turn on debugging.  */
            const IFF_LOOPBACK = 0x8,		/* Is a loopback net.  */
            const IFF_POINTOPOINT = 0x10,	/* Interface is point-to-point link.  */
            const IFF_NOTRAILERS = 0x20,	/* Avoid use of trailers.  */
            const IFF_RUNNING = 0x40,		/* Resources allocated.  */
            const IFF_NOARP = 0x80,		/* No address resolution protocol.  */
            const IFF_PROMISC = 0x100,	/* Receive all packets.  */

            /* Not supported */
            const IFF_ALLMULTI = 0x200,	/* Receive all multicast packets.  */

            const IFF_MASTER = 0x400,		/* Master of a load balancer.  */
            const IFF_SLAVE = 0x800,		/* Slave of a load balancer.  */

            const IFF_MULTICAST = 0x1000,	/* Supports multicast.  */

            const IFF_PORTSEL = 0x2000,	/* Can set media type.  */
            const IFF_AUTOMEDIA = 0x4000,	/* Auto media select active.  */
            const IFF_DYNAMIC = 0x8000	/* Dialup device with changing addresses.  */
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
        let mut params = linux::ifreq::<libc::c_short> {
            name: [0; 16],
            data: linux::IFF_TUN,
        };
        for (from, to) in name.as_bytes().iter().zip(params.name.iter_mut()) {
            *to = *from as libc::c_schar;
        }
        let ret = unsafe {
            linux::tun_create(tun.as_raw_fd(), &params as *const _ as *const libc::c_void as *const i32)
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
        let mut param = linux::ifreq {
            name: name,
            data: addr_to_raw(ip),
        };
        let ret = unsafe {
            linux::add_ip(socket.as_raw_fd(), &mut param as *mut _ as *mut libc::c_void as *mut u8)
        };
        if ret < 0 {
            Err(io::Error::last_os_error()).chain_err(|| ErrorKind::AddIp)?;
        }
        let mut param = linux::ifreq {
            name: name,
            data: addr_to_raw(mask),
        };
        let ret = unsafe {
            linux::add_mask(socket.as_raw_fd(), &mut param as *mut _ as *mut libc::c_void as *mut u8)
        };
        if ret < 0 {
            Err(io::Error::last_os_error()).chain_err(|| ErrorKind::AddIp)?;
        }
        Ok(())
    }

    pub fn set_up(name: [i8; 16]) -> Result<()> {
        let socket = UdpSocket::bind("0.0.0.0:6555").chain_err(|| ErrorKind::AddIp)?;
        let mut param = linux::ifreq {
            name: name,
            data: linux::IFF_UP | linux::IFF_RUNNING,
        };
        let ret = unsafe {
            linux::set_flags(socket.as_raw_fd(), &mut param as *mut _ as *mut libc::c_void as *mut u8)
        };
        if ret < 0 {
            Err(io::Error::last_os_error()).chain_err(|| ErrorKind::AddIp)?;
        }
        Ok(())
    }
}

fn addr_to_raw(addr: IpAddr) -> libc::sockaddr {
    use std::mem::transmute;
    unsafe {
        match addr {
            IpAddr::V4(v4) => {
                transmute(libc::sockaddr_in {
                    sin_family: libc::AF_INET as libc::sa_family_t,
                    sin_addr: libc::in_addr {
                        s_addr: {
                            let bytes = v4.octets();
                            (bytes[3] as u32) << 24 |
                                (bytes[2] as u32) << 16 |
                                (bytes[1] as u32) << 8 |
                                (bytes[0] as u32)
                        }
                    },
                    sin_port: 0,
                    sin_zero: [0; 8],
                })
            }
            IpAddr::V6(_v6) => {
                unimplemented!()
                    //transmute(libc::sockaddr_in6 {
                    //    sin6_family: libc::AF_INET6 as libc::sa_family_t,
                    //    sin6_addr: libc::in6_addr {
                    //        s6_addr: v6.octets(),
                    //        __align: [],
                    //    },
                    //    sin6_port: 0,
                    //    sin6_flowinfo: 0,
                    //    sin6_scope_id: 0,
                    //})
            }
        }
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
