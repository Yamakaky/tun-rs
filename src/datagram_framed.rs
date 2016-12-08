use std::io;
use std::net::SocketAddr;

use tokio_core::io::{Codec, EasyBuf};
use tokio_core::net::UdpCodec;

pub struct Parser;

impl Codec for Parser {
    type In = Vec<u8>;
    type Out = Vec<u8>;

    fn decode(&mut self, buf: &mut EasyBuf) -> io::Result<Option<Vec<u8>>> {
        Ok(Some(buf.as_slice().into()))
    }

    fn encode(&mut self, msg: Vec<u8>, buf: &mut Vec<u8>) -> io::Result<()> {
        buf.extend_from_slice(msg.as_slice());
        Ok(())
    }
}

impl UdpCodec for Parser {
    type In = (SocketAddr, Vec<u8>);
    type Out = (SocketAddr, Vec<u8>);

    fn decode(&mut self, src: &SocketAddr, buf: &[u8]) -> io::Result<Self::In> {
        Ok((src.clone(), buf.into()))
    }

    fn encode(&mut self, msg: Self::Out, buf: &mut Vec<u8>) -> SocketAddr {
        let (addr, data) = msg;
        buf.extend(data);
        addr
    }
}
