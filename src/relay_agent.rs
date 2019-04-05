extern crate tokio;
extern crate futures;

use tokio::prelude::*;
use tokio::io::{copy};
use tokio::net::{TcpListener};

fn main() {
    let addr = "127.0.0.1:1111".parse().unwrap();
    let listener = TcpListener::bind(&addr)
        .expect("unable to bind TCP listener");

    let addr2 = "127.0.0.1:2222".parse().unwrap();
    let listener2 = TcpListener::bind(&addr2)
        .expect("unable to bind TCP listener");

    let servers = listener
        .incoming()
        .zip(listener2
        .incoming())
        .map_err(|err| {
            eprintln!("err {:?}", err)
        })
        .for_each(|(sock1, sock2)| {
            let (reader, writer) = sock1.split();
            let (s_reader, s_writer) = sock2.split();
            let bytes_copied1 = copy(reader, s_writer);
            let bytes_copied2 = copy(s_reader, writer);

            let handle_connections = bytes_copied1.select(bytes_copied2).map(|_amt| {
                println!("wrote bytes")
            }).map_err(|err| {
                eprintln!("IO error {:?}", err)
            });

            tokio::spawn(handle_connections)
        });

        tokio::run(servers);
}