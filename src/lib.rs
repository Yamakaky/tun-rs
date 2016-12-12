#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate error_chain;
extern crate futures;
#[macro_use]
extern crate ioctl;
extern crate libc;
extern crate mio;
extern crate tokio_core;

mod tun;
mod mio_wrapper;

pub use tun::*;
