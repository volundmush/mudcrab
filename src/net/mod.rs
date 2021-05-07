//pub mod telnet;
use mio::net::{TcpListener, TcpStream};
use std::net::SocketAddr;
use std::io::{Result, Write};
use serde::de::Error;
use std::time::Duration;
use mio::{Events, Poll, Token};
use bytes::{Bytes, BytesMut, Buf, BufMut};
use serde_derive::{Serialize, Deserialize};
use rustls::{ServerSession, StreamOwned, ServerConfig, Session};
use std::sync::Arc;
use std::fmt::{Debug, Formatter};

pub mod telnet;
use crate::net::telnet::{TelnetProtocol, TelnetMessage, TelnetEvent, TelnetOption};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Protocol {
    Telnet,
    WebSocket,
    SSH
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConnType {
    Plain,
    TLS
}

#[derive(Debug)]
pub struct ListenerComponent {
    pub listener: TcpListener,
    pub protocol: Protocol,
    pub ctype: ConnType,
    pub token: Token
}

impl ListenerComponent {
    pub fn new(addr: SocketAddr, protocol: Protocol, ctype: ConnType, token: Token) -> Result<Self> {
        let mut listener = TcpListener::bind(addr)?;
        Ok(Self {
            listener,
            protocol,
            ctype,
            token
        })
    }
}


pub enum TransportType {
    TCP(TcpStream),
    TLS(StreamOwned<ServerSession, TcpStream>)
}

impl TransportType {
    pub fn is_tls(&self) -> bool {
        match self {
            Self::TLS(_) => true,
            _ => false
        }
    }
}

impl std::fmt::Debug for TransportType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TCP(stream) => {
                stream.fmt(f)
            },
            Self::TLS(stream) => {
                f.write_str("TLS")
            }
        }
    }
}

impl std::io::Read for TransportType {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        match self {
            TransportType::TCP(stream) => {
                stream.read(buf)
            },
            TransportType::TLS(stream) => {
                stream.read(buf)
            }
        }
    }
}

impl std::io::Write for TransportType {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        match self {
            TransportType::TCP(stream) => {
                stream.write(buf)
            },
            TransportType::TLS(stream) => {
                stream.write(buf)
            }
        }
    }

    fn flush(&mut self) -> Result<()> {
        match self {
            TransportType::TCP(stream) => {
                stream.flush()
            },
            TransportType::TLS(stream) => {
                stream.flush()
            }
        }
    }
}

#[derive(Debug)]
pub enum ConnectionStatus {
    Active,
    ClientEOF,
    ClientTimeout,
    ServerClosed,
    ClientError(std::io::Error)
}

#[derive(Debug)]
pub struct ConnectionComponent {
    pub transport: TransportType,
    pub protocol: Protocol,
    pub addr: SocketAddr,
    pub token: Token,
    pub write_ready: bool,
    pub new_data: bool,
    pub read_buff: BytesMut,
    pub write_buff: BytesMut,
    pub status: ConnectionStatus
}

impl ConnectionComponent {
    pub fn new(stream: TcpStream, addr: SocketAddr, protocol: Protocol, token: Token, tls: Option<Arc<ServerConfig>>) -> Self {

        let transport = if let Some(rc_config) = tls {
            TransportType::TLS(StreamOwned::new(ServerSession::new(&rc_config), stream))
        } else {
            TransportType::TCP(stream)
        };

        Self {
            transport,
            addr,
            protocol,
            token,
            write_ready: false,
            new_data: false,
            read_buff: Default::default(),
            write_buff: Default::default(),
            status: ConnectionStatus::Active
        }
    }
}

impl std::io::Write for ConnectionComponent {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.write_buff.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

impl std::io::Read for ConnectionComponent {
    fn read(&mut self, mut buf: &mut [u8]) -> Result<usize> {
        let written = buf.write(self.read_buff.as_ref())?;
        self.read_buff.advance(written);
        Ok(written)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolCapabilities {
    pub protocol: Protocol,
    pub client_name: String,
    pub client_version: String,
    pub utf8: bool,
    pub html: bool,
    pub mxp: bool,
    pub gmcp: bool,
    pub msdp: bool,
    pub mssp: bool,
    pub ansi: bool,
    pub mtts: bool,
    pub naws: bool,
    pub mccp2: bool,
    pub sga: bool,
    pub linemode: bool,
    pub xterm256: bool,
    pub truecolor: bool,
    pub width: u16,
    pub height: u16,
    pub screen_reader: bool,
    pub vt100: bool,
    pub mouse_tracking: bool,
    pub osc_color_palette: bool,
    pub mnes: bool,
    pub oob: bool,
    pub proxy: bool,
}

impl Default for ProtocolCapabilities {
    fn default() -> Self {
        Self {
            protocol: Protocol::Telnet,
            client_name: "UNKNOWN".to_string(),
            client_version: "UNKNOWN".to_string(),
            utf8: false,
            html: false,
            mxp: false,
            gmcp: false,
            msdp: false,
            mssp: false,
            ansi: false,
            mtts: false,
            naws: false,
            mccp2: false,
            sga: false,
            linemode: false,
            xterm256: false,
            truecolor: false,
            width: 78,
            height: 24,
            screen_reader: false,
            vt100: false,
            mouse_tracking: false,
            osc_color_palette: false,
            mnes: false,
            oob: false,
            proxy: false,
        }
    }
}

impl ProtocolCapabilities {
    pub fn telnet() -> Self {
        ProtocolCapabilities::default()
    }

    pub fn websocket() -> Self {
        let mut out = ProtocolCapabilities::default();
        out.protocol = Protocol::WebSocket;
        out.utf8 = true;
        out.html = true;
        out.gmcp = true;
        out.oob = true;
        out.msdp = true;
        out.ansi = true;
        out.xterm256 = true;
        out.truecolor = true;
        out
    }

    pub fn ssh() -> Self {
        let mut out = ProtocolCapabilities::default();
        out.protocol = Protocol::SSH;
        out.ansi = true;
        out.xterm256 = true;
        out.truecolor = true;
        out
    }
}

#[derive(Debug)]
pub enum ProtocolType {
    Telnet(TelnetProtocol),
    WebSocket,
    SSH
}

#[derive(Debug)]
pub struct ProtocolComponent {
    pub ptype: ProtocolType,
    pub capabilities: ProtocolCapabilities
}

impl ProtocolComponent {
    pub fn telnet(options: Arc<HashMap<u8, TelnetOption>>) -> Self {
        Self {
            ptype: ProtocolType::Telnet(TelnetProtocol::new(options)),
            capabilities: ProtocolCapabilities::telnet()
        }
    }

    pub fn websocket() -> Self {
        Self {
            ptype: ProtocolType::WebSocket,
            capabilities: ProtocolCapabilities::websocket()
        }
    }

    pub fn ssh() -> Self {
        Self {
            ptype: ProtocolType::SSH,
            capabilities: ProtocolCapabilities::ssh()
        }
    }

    pub fn start(&mut self, mut conn: &mut ConnectionComponent) {
        match &mut self.ptype {
            ProtocolType::Telnet(telnet) => {
                telnet.start(conn);
            },
            _ => {

            }
        }
    }

    pub fn process_new_data(&mut self, conn: &mut ConnectionComponent) {
        match &mut self.ptype {
            ProtocolType::Telnet(telnet) => {
                let mut events: Vec<TelnetEvent> = Vec::default();

                while let Some((msg, len)) = TelnetMessage::from_bytes(conn.read_buff.as_ref()) {
                    conn.read_buff.advance(len);
                    telnet.process_message(msg, &mut events, conn, &mut self.capabilities);
                }
            },
            _ => {

            }
        }
    }
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