extern crate bufstream;

use std::str;
use std::str::FromStr;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::net::SocketAddr;
use std::thread::spawn;
use std::sync::mpsc;
use std::sync::mpsc::{Sender, Receiver};
use std::time::Duration;

fn handle_connection(mut stream: TcpStream, sender: Sender<Vec<u8>>, receiver: Receiver<Vec<u8>>) {
    println!("new connection {:?}", stream);

    // create a new connection...
    stream.set_read_timeout(Some(Duration::new(0,0)));
    stream.set_write_timeout(Some(Duration::new(0,0)));
    receiver.recv_timeout(Duration::new(0,0));

    let mut buf: Vec<u8> = vec![0; 1024];

    loop {
        match stream.read(&mut buf) {
            Ok(count) => {
                println!("Read {}, {}", count, String::from_utf8_lossy(&buf));
                match sender.send(buf.clone()) {
                    Ok(_) => println!("sender ok!"),
                    Err(e) => panic!("Err sender send {}", e),
                }
            }
            Err(e) => println!("Error {}", e),
        }

        match stream.write(&receiver.recv().unwrap()) {
            Ok(count) => println!("Wrote {}", count),
            Err(e) => println!("Error {}", e),
        }
    }
}

fn proxy_client_listener(addr: &str, channel_sender: Sender<(Sender<Vec<u8>>, Receiver<Vec<u8>>)>) {
    let listen_addr = SocketAddr::from_str(addr).unwrap();
    let listener = TcpListener::bind(listen_addr).unwrap();

    for stream in listener.incoming() {
        match stream {
            Err(_) => println!("listen error"),
            Ok(mut stream) => {
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
            let s = String::from("hello world!");
            let (sender, receiver) = channel_rx.recv().unwrap();
            match listener.accept() {
                Ok((mut socket, addr)) => {
                    println!("new client: {:?}", addr);
                    spawn(move|| {
                        socket.set_read_timeout(Some(Duration::new(0,0)));
                        socket.set_write_timeout(Some(Duration::new(0,0)));
                        receiver.recv_timeout(Duration::new(0,0));

                        loop {
                            socket.write(&receiver.recv().unwrap()).unwrap();
                            //println!("recved stuff: {}", String::from_utf8_lossy(&receiver.recv().unwrap()));
                            
                            let mut buf: Vec<u8> = vec![0; 1024];
                            match socket.read(&mut buf) {
                                Ok(count) => {
                                    println!("Read {}, {}", count, String::from_utf8_lossy(&buf));
                                    match sender.send(buf.clone()) {
                                        Ok(_) => println!("sender ok!"),
                                        Err(e) => panic!("Err sender send {}", e),
                                    }
                                }
                                Err(e) => println!("Error {}", e),
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
    //spawn(move|| {
    let (channel_tx, channel_rx): (Sender<(Sender<Vec<u8>>, Receiver<Vec<u8>>)>, Receiver<(Sender<Vec<u8>>, Receiver<Vec<u8>>)>) = mpsc::channel();

    proxy_reverse_listener("127.0.0.1:1111", channel_rx); 
    
    proxy_client_listener("127.0.0.1:2222", channel_tx);
    //proxyListener("127.0.0.1:1111");
}