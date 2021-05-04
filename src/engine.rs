use rayon::prelude::*;
use slab::Slab;
use crate::{
    config::{Config},
    net::{Listener, SecureTunnel, Connection, Protocol, ConnType, PollHandler}
};
use mio::{Events, Poll, Token, Interest};
use mio::net::TcpStream;
use std::io::{Result, Read, Write, Error, ErrorKind, copy};
use std::net::SocketAddr;
use bytes::{Bytes, BytesMut, Buf, BufMut};


#[derive(Debug)]
pub struct Engine {
    pub config: Config,
    pub listeners: Slab<Listener>,
    pub connections: Slab<Connection>,
    pub tunnels: Slab<SecureTunnel>,
    pub listen_poll: PollHandler,
    pub conn_poll: PollHandler,
}

impl Engine {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            listeners: Slab::with_capacity(5),
            connections: Slab::with_capacity(10),
            tunnels: Slab::default(),
            listen_poll: PollHandler::new(5, Some((0, 300))).unwrap(),
            conn_poll: PollHandler::new(100, Some((0, 300))).unwrap()
        }
    }

    pub fn register_listener(&mut self, addr: SocketAddr, protocol: Protocol, ctype: ConnType) -> Result<Token> {
        let mut listen = Listener::new(addr, protocol, ctype)?;
        let key = self.listeners.insert(listen);
        if let Some(l) = self.listeners.get_mut(key) {
            self.listen_poll.poller.registry().register(&mut l.listener, Token(key), Interest::READABLE)?;
        }
        println!("Registerd a listener: {}", key);
        Ok(Token(key))
    }

    pub fn setup(&mut self) {
        if let Some(n) = &self.config.net {
            if let Some(l) = &n.listeners {
                let mut l_tokens = Vec::with_capacity(5);
                if let Some(plain_telnet) = &l.plain_telnet {
                    if let Ok(tok) = self.register_listener(plain_telnet.clone(), Protocol::Telnet, ConnType::Plain) {
                        l_tokens.push(tok);
                    } else {
                        panic!("Could not open a listening port for plain telnet!");
                    }

                }

                if l_tokens.is_empty() {
                    panic!("Program has no listeners!");
                }

            } else {
                panic!("Program needs listeners to have meaning!");
            }
        } else {
            panic!("Program requires Net configuration to be useful!");
        }
    }


    fn register_connection(&mut self, stream: TcpStream, addr: SocketAddr, protocol: Protocol, ctype: ConnType) -> Result<()> {
        let mut conn = Connection {
            stream,
            addr,
            protocol: protocol,
            ctype: ctype,
            read_buff: BytesMut::default(),
            write_buff: BytesMut::default(),
            tls: None
        };
        let key = self.connections.insert(conn);
        if let Some(c) = self.connections.get_mut(key) {
            self.conn_poll.poller.registry().register(&mut c.stream, Token(key), Interest::READABLE | Interest::WRITABLE)?;
        }
        Ok(())
    }

    fn run_poll_listeners(&mut self) {
        if self.listen_poll.poll().is_err() {
            panic!("Something went wrong with listen polling!")
        }

        let mut new_conns = Vec::new();

        for event in self.listen_poll.events.iter() {
            let key = event.token().0;
            if let Some(l) = self.listeners.get_mut(key) {
                if let Ok((t, a)) = l.listener.accept() {
                    new_conns.push((t, a, l.protocol.clone(), l.ctype.clone()));
                }
            }
        }

        for (t, a, p, c) in new_conns {
            if let Err(e) = self.register_connection(t, a, p, c) {
                // something done goofed!
                println!("SOmething goofed: {}", e.to_string());
            }
        }

    }

    pub fn run_poll_connections(&mut self) -> (Vec<usize>, Vec<usize>) {
        if self.conn_poll.poll().is_err() {
            panic!("Something went wrong with read polling!")
        }

        let mut read_ready = Vec::new();
        let mut write_ready = Vec::new();

        for event in self.conn_poll.events.iter() {
            let key = event.token().0;
            if event.is_readable() {
                read_ready.push(key);
            }
            if event.is_writable() {
                write_ready.push(key);
            }


        }
        (read_ready, write_ready)

    }

    fn run_conn_reader(&mut self, read_ready: Vec<usize>) {
        let mut read_bucket: [u8; 2048]= [0; 2048];

        for key in read_ready {
            if let Some(l) = self.connections.get_mut(key) {
                loop {
                    match l.stream.read(&mut read_bucket) {
                        Ok(len) => {
                            if len == 0 {
                                // oops, this one disconnected.
                            } else {
                                l.read_buff.extend_from_slice(&read_bucket[..len]);
                            }
                        },
                        Err(e) => {
                            match e.kind() {
                                ErrorKind::WouldBlock => {
                                    // No more bytes available to read.
                                    break;
                                },
                                _ => {
                                    // Will deal with this later
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn run(&mut self) {
        loop {
            self.run_poll_listeners();
            let mut conn_ready = self.run_poll_connections();
            let (read_ready, write_ready) = conn_ready;
            self.run_conn_reader(read_ready);
        }
    }

}