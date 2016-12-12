#[macro_use]
extern crate error_chain;
extern crate futures;
extern crate mio;
extern crate tokio_core;
extern crate tun;

use tokio_core::io::Io;
use tokio_core::reactor;
use futures::Stream;

error_chain! {
    links {
        Tun(tun::tun::Error, tun::tun::ErrorKind);
    }
    foreign_links {
        Io(::std::io::Error);
    }
}

pub const DEFAULT_PORT: u16 = 18424;

pub fn real_main() -> Result<()> {
    let mut core = reactor::Core::new()?;
    let tun = tun::tun::Tun::new("pote", &core.handle())?;
    let stream = tun.framed(tun::datagram_framed::Parser).and_then(|msg| {
        println!("Received {} bytes", msg.len());
        Ok(())
    }).for_each(|_| {
        Ok(())
    });
    core.run(stream)?;
    Ok(())
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
