use std::io;
use std::fs;
use std::os::unix::io::AsRawFd;
use std::os::unix::io::{FromRawFd, IntoRawFd};

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
        Create(code: io::Error) {
            description("Error while creating the device")
            display(me) -> ("{}: {}", me.description(), code)
            cause(code)
            from()
        }
    }
}

mod linux {
    use libc;

    ioctl!(write tun_create with b'T', 202; libc::c_int);

    #[repr(C)]
    pub struct ifreq {
        pub name: [libc::c_char; 16],
        pub flags: libc::c_short,
    }

    pub const IFF_TUN: libc::c_short = 0x0001;
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
        let mut params = linux::ifreq {
            name: [0; 16],
            flags: linux::IFF_TUN,
        };
        for (from, to) in name.as_bytes().iter().zip(params.name.iter_mut()) {
            *to = *from as libc::c_schar;
        }
        let ret = unsafe {
            linux::tun_create(tun.as_raw_fd(), &params as *const _ as *const libc::c_void as *const i32)
        };
        if ret < 0 {
            return Err(ErrorKind::Create(io::Error::last_os_error()))?;
        }
        let mio = unsafe { mio_wrapper::Tun::from_raw_fd(tun.into_raw_fd()) };
        let inner = PollEvented::new(mio,handle)
                // Why From doesn't work here?
                .map_err(ErrorKind::Create)?;
        Ok(Tun {
            name: name.into(),
            inner: inner,
        })
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
}
