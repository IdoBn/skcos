
# skcos

We are building a reverse socks server so the name should obviously be socks backwards ;)
```bash
rev <<< socks
```

## Network Diagram
```
+--------+                            +--------+
|        |                            |        |
|        |       +-----+              |        |
| socks  | ------| nat |---------->   | relay  |
| client |       +-----+              | server |
|        |                            |        |
+--------+                            +--------+
                                          ||
                                          ||
                                          ||
                                      +--------+      +--------------+
                                      |  nat   |      |  another pc  |
                                      +--------+      +--------------+
                                          ||        /
                                          ||       /
                                          ||      /
                                          \/     /
                                      +--------+/
                                      |        |
                                      | reverse|
                                      | socks  | 
                                      | server |
                                      |        |
                                      +--------+
```

## Goal
The goal is to allow the socks client (can be a browser or proxychains) to browse through the reverse socks server

## Usage
first make sure you run the ```relay-agent``` the ```reverse-socks-server``` will connect ot it so it needs to be running.
```bash
cargo run --bin relay-agent
```

Then run the ```reverse-socks-server```
```bash
cargo run --bin reverse-socks-server
```

Finally you can hookup your browser or proxychains to the relay agent and that is it.


### The tests that will be done to prove that this works fully are:
- [x] proxychains + curl
- [ ]  chrome
- [ ]  firefox
- [x]  proxychains + nmap 


## Flow
1. socks client connects to the relay server
2. relay server listens ahead of time to a connection from the socks server.
3. relay server connects the new (incomming socks client connection) to the previously held socks server connection.
    3.1. the two new sockets are somehow mapped together...
3. socks server initiates a new connection to the relay server as soon as it gets data to it's previous connection. this way we can handle multiple connections at once.
4. data is passed between the two sockets until one of them disconnects

## Todo
- [ ] Clean up code after I've properly learned rust.
- [ ] Disconnect dangling sessions (to reproduce this do a simple curl + proxychains and see what happens)