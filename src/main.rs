extern crate bufstream;

use std::str;
use std::str::FromStr;
use std::io::{Read, Write, ErrorKind};
use std::net::{TcpListener, TcpStream};
use std::net::SocketAddr;
use std::thread::spawn;
use std::sync::mpsc;
use std::sync::mpsc::{Sender, Receiver};
use std::time::Duration;

fn handle_connection(mut stream: TcpStream, sender: Sender<Vec<u8>>, receiver: Receiver<Vec<u8>>) {
    println!("new connection {:?}", stream);

    // create a new connection...
    match stream.set_read_timeout(Some(Duration::from_millis(100))) {
        Ok(_) => println!("set read timeout ok"),
        Err(e) => panic!("Set Read Timeout {}", e),
    }

    match stream.set_write_timeout(Some(Duration::from_millis(100))) {
        Ok(_) => println!("set write timeout ok"),
        Err(e) => panic!("Set Write Timeout {}", e),
    }


    loop {
        let mut buf: Vec<u8> = vec![0; 1024];
        match stream.read(&mut buf) {
            Ok(count) => {
                println!("Read {}, {}", count, String::from_utf8_lossy(&buf));
                match sender.send(buf.clone()) {
                    Ok(_) => println!("sender ok!"),
                    Err(e) => panic!("Err sender send {}", e),
                }
            }
            Err(e) => match e.kind() {
                ErrorKind::WouldBlock => (),
                _ => panic!("Error read {}", e),
            }
        }

        match receiver.recv_timeout(Duration::from_millis(100)) {
            Ok(r) => {
                println!("recv timeout ok");
                match stream.write_all(&r) {
                    Ok(()) => println!("Wrote"),
                    Err(e) => println!("Error write {}", e),
                }
            }
            Err(_e) => (),
        }
    }
}

fn proxy_client_listener(addr: &str, channel_sender: Sender<(Sender<Vec<u8>>, Receiver<Vec<u8>>)>) {
    let listen_addr = SocketAddr::from_str(addr).unwrap();
    let listener = TcpListener::bind(listen_addr).unwrap();

    for stream in listener.incoming() {
        match stream {
            Err(_) => println!("listen error"),
            Ok(stream) => {
                println!("connection from {} to {}",
                         stream.peer_addr().unwrap(),
                         stream.local_addr().unwrap());
                         let (proxy_tx, proxy_rx): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = mpsc::channel();
                         let (client_tx, client_rx): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = mpsc::channel();
                         spawn(move|| {
                             handle_connection(stream, client_tx, proxy_rx);
                         });
                        channel_sender.send((proxy_tx, client_rx)).unwrap();
                }
        }
    }  
}

fn proxy_reverse_listener(addr: &str, channel_rx: Receiver<(Sender<Vec<u8>>, Receiver<Vec<u8>>)>) {
    let listen_addr = SocketAddr::from_str(addr).unwrap();
    let listener = TcpListener::bind(listen_addr).unwrap();

    spawn(move|| {
        loop {
            let (sender, receiver) = channel_rx.recv().unwrap();
            match listener.accept() {
                Ok((mut socket, addr)) => {
                    println!("new client: {:?}", addr);
                    spawn(move|| {
                        match socket.set_read_timeout(Some(Duration::from_millis(100))) {
                            Ok(_) => println!("set read timeout ok"),
                            Err(e) => panic!("Set Read Timeout {}", e),
                        }

                        match socket.set_write_timeout(Some(Duration::from_millis(100))) {
                            Ok(_) => println!("set read timeout ok"),
                            Err(e) => panic!("Set Write Timeout {}", e),
                        }

                        loop {
                            match receiver.recv_timeout(Duration::from_millis(100)) {
                                Ok(r) => {
                                    println!("recv timeout ok");
                                    match socket.write_all(&r) {
                                        Ok(()) => println!("Wrote"),
                                        Err(e) => println!("Error write {}", e),
                                    }
                                }
                                Err(_e) => (),
                            }
                            
                            let mut buf: Vec<u8> = vec![0; 1024];
                            match socket.read(&mut buf) {
                                Ok(count) => {
                                    println!("Read {}, {}", count, String::from_utf8_lossy(&buf));
                                    match sender.send(buf.clone()) {
                                        Ok(_) => println!("sender ok!"),
                                        Err(e) => panic!("Err sender send {}", e),
                                    }
                                }
                                Err(e) => match e.kind() {
                                    ErrorKind::WouldBlock => (),
                                    _ => panic!("Error read {}", e),
                                }
                            }
                        }

                    });
                }
                Err(e) => println!("couldn't get client: {:?}", e),
            }
        }        
    });
}

fn main() {
    let (channel_tx, channel_rx): (Sender<(Sender<Vec<u8>>, Receiver<Vec<u8>>)>, Receiver<(Sender<Vec<u8>>, Receiver<Vec<u8>>)>) = mpsc::channel();

    proxy_reverse_listener("127.0.0.1:1111", channel_rx); 
    
    proxy_client_listener("127.0.0.1:2222", channel_tx);
}