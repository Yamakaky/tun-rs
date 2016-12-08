use std::io;
use std::os::unix::io::{RawFd, FromRawFd};

use libc::{self, c_void};
use mio::{Poll, Token, Ready, PollOpt, Evented};
use mio::unix::EventedFd;

pub struct Tun(RawFd);
pub struct Tap(RawFd);

impl FromRawFd for Tun {
    unsafe fn from_raw_fd(fd: RawFd) -> Tun {
        Tun(fd)
    }
}


impl Evented for Tun {
    fn register(&self, poll: &Poll, token: Token, interest: Ready,
        opts: PollOpt)
        -> io::Result<()>
    {
        EventedFd(&self.0).register(poll, token, interest, opts)
    }

    fn reregister(&self, poll: &Poll, token: Token, interest: Ready,
        opts: PollOpt)
        -> io::Result<()>
    {
        EventedFd(&self.0).reregister(poll, token, interest, opts)
    }

    fn deregister(&self, poll: &Poll) -> io::Result<()> {
        EventedFd(&self.0).deregister(poll)
    }
}

impl Drop for Tun {
    fn drop(&mut self) {
        unsafe {
            libc::close(self.0);
        }
    }
}

impl FromRawFd for Tap {
    unsafe fn from_raw_fd(fd: RawFd) -> Tap {
        Tap(fd)
    }
}

impl Evented for Tap {
    fn register(&self, poll: &Poll, token: Token, interest: Ready,
        opts: PollOpt)
        -> io::Result<()>
    {
        EventedFd(&self.0).register(poll, token, interest, opts)
    }

    fn reregister(&self, poll: &Poll, token: Token, interest: Ready,
        opts: PollOpt)
        -> io::Result<()>
    {
        EventedFd(&self.0).reregister(poll, token, interest, opts)
    }

    fn deregister(&self, poll: &Poll) -> io::Result<()> {
        EventedFd(&self.0).deregister(poll)
    }
}

impl Drop for Tap {
    fn drop(&mut self) {
        unsafe {
            libc::close(self.0);
        }
    }
}

impl io::Read for Tun {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let ret = unsafe {
            libc::read(self.0,
                       buf.as_mut_ptr() as *mut c_void,
                       buf.len())
        };
        if ret < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(ret as usize)
        }
    }
}

impl io::Write for Tun {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let ret = unsafe {
            libc::write(self.0,
                        buf.as_ptr() as *const c_void,
                        buf.len())
        };
        if ret < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(ret as usize)
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
