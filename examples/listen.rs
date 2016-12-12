#[macro_use]
extern crate error_chain;
extern crate futures;
extern crate mio;
#[macro_use]
extern crate tokio_core;
extern crate tun;

use std::io::Read;

use tokio_core::reactor;
use futures::Async;

error_chain! {
    links {
        Tun(tun::Error, tun::ErrorKind);
    }
    foreign_links {
        Io(::std::io::Error);
    }
}

struct Server {
    tun: tun::Tun,
}

impl futures::Future for Server {
    type Item = ();
    type Error = Error;

    fn poll(&mut self) -> Result<Async<()>> {
        loop {
            let mut buf = [0; 1500];
            let nb_bytes = try_nb!(self.tun.read(&mut buf));
            println!("Received {} bytes", nb_bytes);
        }
    }
}

pub const DEFAULT_PORT: u16 = 18424;

pub fn real_main() -> Result<()> {
    let mut core = reactor::Core::new()?;
    let tun = tun::Tun::new("pote", &core.handle())?;
    Ok(core.run(Server {
        tun: tun
    })?)
}

pub fn main() {
    if let Err(e) = real_main() {
        println!("{}", e);
        for cause in e.iter().skip(1) {
            println!("  caused by: {}", cause);
        }
        if let Some(b) = e.backtrace() {
            println!("{:?}", b);
        }
    }
}
