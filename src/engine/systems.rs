use legion::*;
use crate::engine::resources::{ListenPoll, ConnPoll, TelnetOptions};
use crate::net::{ConnectionComponent, ListenerComponent, TransportType,
                 Protocol, ConnType, ConnectionStatus, ProtocolComponent};
use std::io::{Error, ErrorKind, Read, Write};
use legion::systems::CommandBuffer;
use mio::{Events, Poll, Token, Interest};
use mio::net::TcpStream;
use bytes::Buf;

#[system]
pub fn poll_listeners(#[resource] lis_poll: &mut ListenPoll) {
    lis_poll.poll();
}

#[system(for_each)]
pub fn accept_new_connections(cmd: &mut CommandBuffer, lis: &mut ListenerComponent, #[resource] lis_poll: &mut ListenPoll, #[resource] con_poll: &mut ConnPoll, #[resource] tel_opts: &TelnetOptions) {
    if !lis_poll.accept_ready.contains(&lis.token) {
        return
    }

    loop {
        match lis.listener.accept() {
            Ok((mut t, a)) => {
                let tok = con_poll.get_next();
                if let Err(e) = con_poll.handler.poller.registry().register(&mut t, tok.clone(), Interest::READABLE | Interest::WRITABLE) {
                    panic!("Something going wrong with conn poll!");
                } else {
                    let mut conn = match lis.ctype {
                        ConnType::Plain => ConnectionComponent::new(t, a, lis.protocol.clone(), tok, None),
                        ConnType::TLS => ConnectionComponent::new(t, a, lis.protocol.clone(), tok, None)
                    };
                    let mut prot = match lis.protocol {
                        Protocol::Telnet => ProtocolComponent::telnet(tel_opts.0.clone()),
                        Protocol::WebSocket => ProtocolComponent::websocket(),
                        Protocol::SSH => ProtocolComponent::ssh()
                    };
                    prot.start(&mut conn);
                    cmd.push((conn, prot));
                }
            },
            Err(e) => {
                match e.kind() {
                    ErrorKind::WouldBlock => {
                        break;
                    },
                    _ => {
                        break;
                    }
                }
            }
        }
    }
}

#[system]
pub fn poll_connections(#[resource] conn_poll: &mut ConnPoll) {
    conn_poll.poll();
}

#[system(for_each)]
pub fn process_connection_read(ent: &Entity, conn: &mut ConnectionComponent,
                               prot: &mut ProtocolComponent, #[resource] conn_poll: &ConnPoll) {
    if !conn_poll.read_ready.contains(&conn.token) {
        return
    }

    let mut total_bytes: usize = 0;
    let mut read_bucket: [u8; 2048] = [0; 2048];

    loop {
        match conn.transport.read(&mut read_bucket) {
            Ok(len) => {
                if len == 0 {
                    conn.status = ConnectionStatus::ClientEOF;
                    break;
                } else {
                    total_bytes += len;
                    let new_bytes = &read_bucket[..len];
                    conn.read_buff.extend_from_slice(new_bytes);
                }
            },
            Err(e) => {
                match e.kind() {
                    ErrorKind::WouldBlock => {
                        // No more bytes available to read. This is good.
                        break;
                    },
                    _ => {
                        conn.status = ConnectionStatus::ClientError(e);
                        break;
                    }
                }
            }
        }
    }
    if total_bytes > 0 {
        conn.new_data = true;
    }
}

#[system(par_for_each)]
pub fn process_connection_newdata(conn: &mut ConnectionComponent, prot: &mut ProtocolComponent) {
    if conn.new_data {
        prot.process_new_data(conn);
        conn.new_data = false;
    }
}

#[system(par_for_each)]
pub fn process_connection_outgoing(conn: &mut ConnectionComponent, #[resource] conn_poll: &ConnPoll) {

    if !conn_poll.write_ready.contains(&conn.token) {
        return;
    }

    while !conn.write_buff.is_empty() {
        match conn.transport.write(conn.write_buff.as_ref()) {
            Ok(len) => {
                conn.write_buff.advance(len);
            },
            Err(e) => {
                match e.kind() {
                    ErrorKind::WouldBlock => {
                        break;
                    },
                    _ => {
                        break;
                    }
                }
            }
        }
    }

}