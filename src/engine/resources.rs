use crate::net::{PollHandler, Protocol, ConnType, TransportType};
use mio::net::TcpStream;
use std::net::SocketAddr;
use mio::{Token};
use std::io::{Result, Error};
use std::cmp::max;
use crate::net::telnet::{TelnetOption};
use crate::net::telnet::codes as tc;
use std::collections::{HashMap};
use std::sync::Arc;

pub struct TelnetOptions(pub Arc<HashMap<u8, TelnetOption>>);

impl Default for TelnetOptions {
    fn default() -> Self {
        let mut map: HashMap<u8, TelnetOption> = Default::default();

        map.insert(tc::SGA, TelnetOption {allow_local: true, allow_remote: true, start_remote: false, start_local: true});
        map.insert(tc::NAWS, TelnetOption {allow_local: false, allow_remote: true, start_remote: true, start_local: false});
        map.insert(tc::MTTS, TelnetOption {allow_local: false, allow_remote: true, start_remote: true, start_local: false});
        //map.insert(tc::MXP, TelnetOption {allow_local: true, allow_remote: true, start_remote: false, start_local: true});
        map.insert(tc::MSSP, TelnetOption {allow_local: true, allow_remote: true, start_remote: false, start_local: true});
        //map.insert(tc::MCCP2, TelnetOption {allow_local: true, allow_remote: true, start_remote: false, start_local: true});
        //map.insert(tc::MCCP3, TelnetOption {allow_local: true, allow_remote: true, start_remote: false, start_local: true});
        map.insert(tc::GMCP, TelnetOption {allow_local: true, allow_remote: true, start_remote: false, start_local: true});
        map.insert(tc::MSDP, TelnetOption {allow_local: true, allow_remote: true, start_remote: false, start_local: true});
        map.insert(tc::LINEMODE, TelnetOption {allow_local: false, allow_remote: true, start_remote: true, start_local: false});
        map.insert(tc::TELOPT_EOR, TelnetOption {allow_local: true, allow_remote: true, start_remote: false, start_local: true});

        Self(Arc::new(map))
    }
}

pub struct ConnPoll {
    pub handler: PollHandler,
    pub write_ready: Vec<Token>,
    pub read_ready: Vec<Token>,
    pub next: usize
}

impl ConnPoll {

    pub fn new(handler: PollHandler) -> Self {
        return Self {
            handler,
            write_ready: Default::default(),
            read_ready: Default::default(),
            next: 0
        }
    }

    pub fn poll(&mut self) -> usize {
        if self.handler.poll().is_err() {
            panic!("Something went wrong with connection polling!")
        }
        self.write_ready.clear();
        self.read_ready.clear();

        for event in self.handler.events.iter() {
            let key = event.token();
            if event.is_readable() {
                self.read_ready.push(key);
            }
            if event.is_writable() {
                println!("{} is writable!", key.0);
                self.write_ready.push(key);
            }
        }
        max(self.write_ready.len(), self.read_ready.len())
    }

    pub fn get_next(&mut self) -> Token {
        self.next = self.next + 1;
        Token(self.next)
    }

}


pub struct ListenPoll {
    pub handler: PollHandler,
    pub conns: Vec<(TcpStream, SocketAddr, Protocol, ConnType)>,
    pub accept_ready: Vec<Token>,
    pub next: usize
}

impl ListenPoll {
    pub fn new(handler: PollHandler) -> Self {
        Self {
            handler,
            conns: Default::default(),
            accept_ready: Default::default(),
            next: 0
        }
    }

    pub fn get_next(&mut self) -> Token {
        self.next = self.next + 1;
        Token(self.next)
    }

    pub fn poll(&mut self) -> usize {
        if self.handler.poll().is_err() {
            panic!("Something went wrong with listen polling!")
        }
        self.accept_ready.clear();

        for event in self.handler.events.iter() {
            let key = event.token();
            if event.is_readable() {
                self.accept_ready.push(key);
            }
        }
        self.accept_ready.len()
    }
}
