pub mod telnet;
use mio::net::{TcpListener, TcpStream};
use std::net::SocketAddr;
use std::io::Result;
use serde::de::Error;
use std::time::Duration;
use mio::{Events, Poll};
use bytes::{Bytes, BytesMut, Buf, BufMut};

#[derive(Debug, Clone)]
pub enum Protocol {
    Telnet,
    WebSocket,
    SSH
}

#[derive(Debug, Clone)]
pub enum ConnType {
    Plain,
    TLS
}

#[derive(Debug)]
pub struct Listener {
    pub listener: TcpListener,
    pub protocol: Protocol,
    pub ctype: ConnType
}

impl Listener {
    pub fn new(addr: SocketAddr, protocol: Protocol, ctype: ConnType) -> Result<Self> {
        let mut listener = TcpListener::bind(addr)?;
        Ok(Self {
            listener,
            protocol,
            ctype
        })
    }
}

#[derive(Debug)]
pub struct Connection {
    pub stream: TcpStream,
    pub protocol: Protocol,
    pub ctype: ConnType,
    pub addr: SocketAddr,
    pub tls: Option<usize>,
    pub read_buff: BytesMut,
    pub write_buff: BytesMut
}

#[derive(Debug)]
pub struct SecureTunnel {
    pub conn: usize
}

#[derive(Debug)]
pub struct PollHandler {
    pub poller: Poll,
    pub duration: Option<Duration>,
    pub events: Events
}

impl PollHandler {
    pub fn new(capacity: usize, dur_time: Option<(u64, u32)>) -> Result<Self> {
        let mut poller = Poll::new()?;
        let duration = if let Some((secs, nanos)) = dur_time {
            Some(Duration::new(secs, nanos))
        } else {
            None
        };
        Ok(Self {
            poller,
            duration,
            events: Events::with_capacity(capacity)
        })
    }

    pub fn poll(&mut self) -> Result<()> {
        self.poller.poll(&mut self.events, self.duration)
    }
}