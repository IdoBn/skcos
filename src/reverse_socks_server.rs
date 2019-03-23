/*
Design:
Here we will need two sockets

socket <--- socks server ---> new connection.

1. we connect to our relay agent.
2. we parse the socks that was passed to us via the relay server
3. we connect to the socket that was requested
4. we connect again on a new thread (to support multiple connection.)
*/

use std::io::prelude::*;
use std::io::ErrorKind::{ConnectionRefused, ConnectionReset, WouldBlock};
use std::net::{TcpStream, Ipv4Addr, Shutdown};
use std::time::Duration;
use std::thread::spawn;

extern crate bincode;
#[macro_use]
extern crate serde_derive;
extern crate serde;

#[derive(Debug, Serialize, Deserialize)]
struct SocksConnect {
    ver: u8,
    nmethods: u8,
    method: u8,
}

#[derive(Debug, Serialize, Deserialize)]
struct SocksSelectedMethod {
    ver: u8,
    method: u8,
}

#[derive(Debug, Serialize, Deserialize)]
struct SocksRequest {
    ver: u8,
    command: u8,
    reserved: u8,
    addr_type: u8,
    remote_addr: Ipv4Addr,
    port: u16,
}

#[derive(Debug, Serialize, Deserialize)]
struct SocksSuccess {
    ver: u8,
    result: u8,
    reserved: u8,
    addr_type: u8,
    remote_addr: u32,
    port: u16,
}

impl Default for SocksSuccess {
    fn default() -> Self {
        SocksSuccess {
            ver:         5,
            result:      0,
            reserved:    0,
            addr_type:   1,
            remote_addr: 0,
            port:        0,
        }
    }
}

fn handle_socks(addr: &str) -> std::io::Result<()>  {
    let mut stream = TcpStream::connect(addr)?;

    let mut buf: Vec<u8> = vec![0; 3];
    println!("reading {}", buf.len());
    stream.read_exact(&mut buf).unwrap();
    println!("done reading {:?}", buf);
    let socks_connect = SocksConnect { 
        ver: buf[0], 
        nmethods: buf[1], 
        method: buf[2] 
    };

    println!("connected {:?}", socks_connect);

    let socks_selected_method = SocksSelectedMethod {
        ver: socks_connect.ver,
        method: socks_connect.method,
    };
    match bincode::serialize(&socks_selected_method) {
        Ok(bytes) => stream.write_all(&bytes).unwrap(),
        Err(e) => println!("error {}", e),
    }

    let mut buf2: Vec<u8> = vec![0; 10];
    stream.read_exact(&mut buf2).unwrap();

    let socks_request = SocksRequest {
        ver:            buf2[0],
        command:        buf2[1],
        reserved:       buf2[2],
        addr_type:      buf2[3],
        remote_addr:    Ipv4Addr::new(buf2[4],buf2[5],buf2[6],buf2[7]),
        port:           ((buf2[8] as u16) << 8) | (buf2[9] as u16), 
    };

    println!("socket request {:?}", socks_request);

    match bincode::serialize(&SocksSuccess::default()) {
        Ok(bytes) => stream.write_all(&bytes).unwrap(),
        Err(e) => println!("error {}", e),
    }

    match TcpStream::connect(format!("{}:{}", 
                                    socks_request.remote_addr, 
                                    socks_request.port)) {
        Ok(mut internal_stream) => {
            println!("internal stream {:?}", internal_stream);

            spawn(move|| {
                bind_sockets(&mut stream, &mut internal_stream);
            });
        }
        Err(e) => match e.kind() {
            ConnectionRefused => {
                println!("Connection Refused");
                drop(stream)
                //stream.shutdown(Shutdown::Both).unwrap();
            }
            ConnectionReset => println!("Connection Reset"),
            _ => panic!("error, {}, {:?}", e, e.kind()),
        }
    }


    Ok(())
}

fn bind_sockets(external: &mut TcpStream, internal: &mut TcpStream) {
    match external.set_read_timeout(Some(Duration::from_millis(100))) {
        Ok(_) => println!("set read timeout ok"),
        Err(e) => panic!("Set Read Timeout {}", e),
    }

    match external.set_write_timeout(Some(Duration::from_millis(100))) {
        Ok(_) => println!("set write timeout ok"),
        Err(e) => panic!("Set Write Timeout {}", e),
    }
    
    match internal.set_read_timeout(Some(Duration::from_millis(100))) {
        Ok(_) => println!("set read timeout ok"),
        Err(e) => panic!("Set Read Timeout {}", e),
    }

    match internal.set_write_timeout(Some(Duration::from_millis(100))) {
        Ok(_) => println!("set write timeout ok"),
        Err(e) => panic!("Set Write Timeout {}", e),
    }

    println!("external {:?}", external);
    loop {
        let mut buf: Vec<u8> = vec![0; 1024];
        match external.read(&mut buf) {
            Ok(count) => {
                match internal.write_all(&buf[..count]) {
                    Ok(_) => {
                        println!("sender ok!");
                    }
                    Err(_e) => break,
                }
            }
            Err(e) => match e.kind() {
                WouldBlock => (),
                ConnectionReset => break,
                _ => panic!("Error read {}", e),
            }
        }

        buf = vec![0; 1024];
        match internal.read(&mut buf) {
            Ok(count) => {
                println!("Read {}, {}", count, String::from_utf8_lossy(&buf));
                match external.write_all(&buf[..count]) {
                    Ok(_) => println!("sender ok!"),
                    Err(_e) => break,
                }
            }
            Err(e) => match e.kind() {
                WouldBlock => (),
                ConnectionReset => break,
                _ => panic!("Error read {}", e),
            }
        } 
    }
}

fn main() {
    // connect to relay agent
    loop {
        handle_socks("127.0.0.1:1111").unwrap();
    }
    
}