/*
Design:
Here we will need two sockets

socket <--- socks server ---> new connection.

1. we connect to our relay agent.
2. we parse the socks that was passed to us via the relay server
3. we connect to the socket that was requested
4. we connect again on a new thread (to support multiple connection.)
*/

extern crate tokio;
extern crate futures;

use std::net::{Ipv4Addr, SocketAddrV4, SocketAddr, IpAddr};


use tokio::prelude::*;
use tokio::io::{copy, read_exact, write_all};
use tokio::net::{TcpStream};




use futures::future;
use futures::{Future};
use std::io::{Error, ErrorKind};


// Various constants associated with the SOCKS protocol

#[allow(dead_code)]
mod v5 {
    pub const VERSION: u8 = 5;

    pub const METH_NO_AUTH: u8 = 0;
    pub const METH_GSSAPI: u8 = 1;
    pub const METH_USER_PASS: u8 = 2;

    pub const CMD_CONNECT: u8 = 1;
    pub const CMD_BIND: u8 = 2;
    pub const CMD_UDP_ASSOCIATE: u8 = 3;

    pub const ATYP_IPV4: u8 = 1;
    pub const ATYP_IPV6: u8 = 4;
    pub const ATYP_DOMAIN: u8 = 3;
}

#[allow(dead_code)]
mod v4 {
    pub const VERSION: u8 = 4;

    pub const CMD_CONNECT: u8 = 1;
    pub const CMD_BIND: u8 = 2;
}

fn main() {
    // connect to relay agent
    loop {

        let addr = "127.0.0.1:1111".parse().unwrap();
        let fut = TcpStream::connect(&addr)
            .and_then(|stream| {
                read_exact(stream, [0u8])
            })
            .and_then(|(stream, buf)|{
                match buf[0] {
                    v5::VERSION => future::ok(stream),
                    v4::VERSION => future::err(Error::new(ErrorKind::Other, "oh no!")),

                    // If we hit an unknown version, we return a "terminal future"
                    // which represents that this future has immediately failed. In
                    // this case the type of the future is `io::Error`, so we use a
                    // helper function, `other`, to create an error quickly.
                    _ => future::err(Error::new(ErrorKind::Other, "oh no!")),
                }
            })
            .and_then(|stream| {
                // num methods
                read_exact(stream, [0u8])
            })
            .and_then(|(stream, buf)| {
                // read number of methods
                read_exact(stream, vec![0u8; buf[0] as usize])
            })
            .and_then(|(stream, buf)| {
                if buf.contains(&v5::METH_NO_AUTH) {
                    future::ok(stream)
                } else {
                    future::err(Error::new(ErrorKind::Other, "no supported method given"))
                }
            })
            .and_then(|stream| {
                write_all(stream, [v5::VERSION, v5::METH_NO_AUTH])
            })
            // read type
            .and_then(|(stream, _buf)| {
                read_exact(stream, [0u8])
            })
            .and_then(|(stream, buf)| {
                if buf[0] == v5::VERSION {
                    future::ok(stream)
                } else {
                    future::err(Error::new(ErrorKind::Other,  "didn't confirm with v5 version"))
                }
            })
            // read method
            .and_then(|stream| {
                read_exact(stream, [0u8])
            })
            .and_then(|(stream, buf)| {
                if buf[0] == v5::CMD_CONNECT {
                    future::ok(stream)
                } else {
                    future::err(Error::new(ErrorKind::Other,"unsupported command"))
                }
            })
            // reserved
            .and_then(|c| read_exact(c, [0u8]).map(|c| c.0))
            // addr type
            .and_then(|c| read_exact(c, [0u8]))
            .and_then(|(c, _buf)| {
                read_exact(c, [0u8; 6])
            })
            .and_then(|(c, buf)| {
                let addr = Ipv4Addr::new(buf[0], buf[1], buf[2], buf[3]);
                let port = ((buf[4] as u16) << 8) | (buf[5] as u16);
                let addr = SocketAddrV4::new(addr, port);
                future::ok((c, SocketAddr::V4(addr)))
            })
            .and_then(|(c, addr)| {

                TcpStream::connect(&addr)
                    .map(move |c2| {
                       (c, c2, addr) 
                    })
                    .map_err(|err| {
                        eprintln!("tcp connect error {:?}", err);
                        err
                    })
            })
            .and_then(|(c, c2, addr)| {
               let mut resp = [0u8; 10];

                // VER - protocol version
                resp[0] = 5;

                // REP - "reply field" -- what happened with the actual connect.
                //
                // In theory this should reply back with a bunch more kinds of
                // errors if possible, but for now we just recognize a few concrete
                // errors.
                resp[1] = 0;

                // RSV - reserved
                resp[2] = 0;

                // ATYP, BND.ADDR, and BND.PORT
                //
                // These three fields, when used with a "connect" command
                // (determined above), indicate the address that our proxy
                // connection was bound to remotely. There's a variable length
                // encoding of what's actually written depending on whether we're
                // using an IPv4 or IPv6 address, but otherwise it's pretty
                // standard.
                
                
                resp[3] = 1;
                let ip_octets = match addr.ip() {
                    IpAddr::V4(ip4) => ip4.octets(),
                    IpAddr::V6(_) => [0u8, 0u8, 0u8, 0u8], 
                };
                resp[4..8].copy_from_slice(&ip_octets);
                let pos = 8;
                resp[pos] = (addr.port() >> 8) as u8;
                resp[pos + 1] = addr.port() as u8;

                // Slice our 32-byte `resp` buffer to the actual size, as it's
                // variable depending on what address we just encoding. Once that's
                // done, write out the whole buffer to our client.
                //
                // The returned type of the future here will be `(TcpStream,
                // TcpStream)` representing the client half and the proxy half of
                // the connection.;
                write_all(c, resp)
                    .map(move |(c1, _)| {
                        (c1, c2)
                    })
            })
            .and_then(|(c, c2)| {
                let (reader, writer) = c.split();
                let (reader2, writer2) = c2.split();

                let bytes_copied1 = copy(reader, writer2);
                let bytes_copied2 = copy(reader2, writer);

                let handle_connections = bytes_copied1.select(bytes_copied2).map(|_amt| {
                    println!("wrote bytes hello")
                }).map_err(|err| {
                    eprintln!("IO error {:?}", err)
                });

                tokio::spawn(handle_connections);
                Ok(())
            })
            .or_else(|err| {
                eprintln!("err {:?}", err);
                Ok(())
            });

            tokio::run(fut);
    }

}