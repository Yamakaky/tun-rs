use std::net::*;

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

pub fn addr_to_raw(addr: IpAddr) -> libc::sockaddr {
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
