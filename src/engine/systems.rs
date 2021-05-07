use legion::*;
use crate::engine::Delta;
use crate::engine::resources::{ListenPoll, ConnPoll, TelnetOptions};
use crate::net::{ConnectionComponent, ListenerComponent, TransportType, ProtocolStatus,
                 Protocol, ConnType, ConnectionStatus, ProtocolComponent, ProtocolType,
                 ProtocolEvent, ProtocolOutEvent};
use std::io::{Error, ErrorKind, Read, Write};
use legion::systems::CommandBuffer;
use mio::{Events, Poll, Token, Interest};
use mio::net::TcpStream;
use bytes::Buf;
use std::time::{Duration, Instant};
use crate::game::objects::{MudSession};
use std::collections::{VecDeque, HashSet, HashMap};
use crate::game::process::ProcessComponent;
use crate::game::login_cmds::{LoginCommands};
use crate::game::resources::{ProcessCounter, ProcessIndex, PendingUserCreations};

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

    if conn_poll.write_ready.contains(&conn.token) {
        conn.write_ready = true;
    }

    if !conn.write_ready {
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
                        conn.write_ready = false;
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

#[system(par_for_each)]
pub fn connection_health_check(conn: &mut ConnectionComponent, prot: &mut ProtocolComponent) {
    prot.health_check(conn);
}


#[system(for_each)]
pub fn execute_connection_events(ent: &Entity, conn: &mut ConnectionComponent, prot: &mut ProtocolComponent, #[resource] lcmds: &mut LoginCommands) {
    if prot.session.is_some() {
        return
    }

    if let Some(ev) = prot.in_buffer.pop_front() {

        match ev {
            ProtocolEvent::Line(s) => {
                if let Some(user) = prot.user {
                    println!("This should not happen yet!");
                    // TODO: this will call the function for running a menu screen command.
                } else {
                    lcmds.execute(prot, s);
                }
            },
            ProtocolEvent::OOB(cmd, args, kwargs) => {

            },
            ProtocolEvent::RequestMSSP => {
                // TODO: This should render MSSP data to a HashMap and push it to msess.out_events
            },
            ProtocolEvent::CreateUser(user, pass) => {

            },
            ProtocolEvent::Login(user, pass) => {

            }
        }
    }
}


#[system(for_each)]
pub fn transfer_events(cmd: &mut CommandBuffer, wrl: &mut World, msess: &mut MudSession) {
    for ent in msess.connections.iter() {
        if let Ok(mut entry) = wrl.entry_mut(*ent) {
            if let Ok(mut prot) = entry.get_component_mut::<ProtocolComponent>() {
                for ev in prot.in_buffer.iter() {
                    msess.in_events.push_back(ev.clone());
                }
                prot.in_buffer.clear();

                for ev in msess.out_events.iter() {
                    prot.out_buffer.push_back(ev.clone());
                }
            }
        }
    }
    msess.out_events.clear();
}

#[system(par_for_each)]
pub fn send_out_events(prot: &mut ProtocolComponent, conn: &mut ConnectionComponent) {
    while let Some(ev) = prot.out_buffer.pop_front() {
        prot.send_event(ev, conn);
    }
}

#[system(for_each)]
pub fn session_in_events(cmd: &mut CommandBuffer, msess: &mut MudSession, #[resource] pid: &mut ProcessCounter, #[resource] pdx: &mut ProcessIndex) {
    // Pop an event off of MudSession and execute it, if applicable.
    if let Some(ev) = msess.in_events.pop_front() {
        match ev {
            ProtocolEvent::Line(s) => {
                println!("Got a process command: {}", s);
                pid.0 += 1;
                let mut process = ProcessComponent::from_command(msess, pid.0, s);
                let mut ent = cmd.push((process, ));
                pdx.0.insert(pid.0, ent);
            },
            ProtocolEvent::OOB(cmd, args, kwargs) => {

            },
            ProtocolEvent::RequestMSSP => {
                // TODO: This should render MSSP data to a HashMap and push it to msess.out_events
            },
            _ => {

            }
        }
    }
}

#[system(for_each)]
pub fn execute_process(ent: &Entity, proc: &mut ProcessComponent, wrl: &mut World, cmd: &mut CommandBuffer) {
    println!("FOUND A PROCESS: {:?}", proc);
}