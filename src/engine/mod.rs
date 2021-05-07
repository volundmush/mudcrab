mod systems;
mod resources;

use legion::*;
use crate::{
    config::{Config},
    net::{ListenerComponent, ConnectionComponent,
          ProtocolComponent, Protocol, ConnType, PollHandler}
};
use mio::{Events, Poll, Token, Interest};
use mio::net::TcpStream;
use std::io::{Result, Read, Write, Error, ErrorKind, copy};
use std::net::SocketAddr;
use std::collections::{HashMap};
use bytes::{Bytes, BytesMut, Buf, BufMut};
use std::thread::{sleep, yield_now};
use std::time::{Duration, Instant};

use crate::engine::resources::{
    ConnPoll, ListenPoll, TelnetOptions
};


use crate::engine::systems::{
    poll_listeners_system, accept_new_connections_system,
    poll_connections_system, process_connection_read_system, connection_health_check_system,
    process_connection_newdata_system, process_connection_outgoing_system
};


pub struct Delta(Duration);


pub struct Engine {
    pub config: Config,
    pub world: World,
    pub resources: Resources,
}

impl Engine {
    pub fn new(config: Config) -> Self {

        let group_1 = <(ConnectionComponent, ProtocolComponent)>::to_group();

        let listen_poll = ListenPoll::new(PollHandler::new(5, Some((0, 300))).unwrap());
        let conn_poll = ConnPoll::new(PollHandler::new(100, Some((0, 300))).unwrap());

        let mut resources = Resources::default();
        resources.insert(listen_poll);
        resources.insert(conn_poll);
        resources.insert(TelnetOptions::default());

        let w_options = WorldOptions {
            groups: vec![group_1],
        };

        let mut world = World::new(w_options);
        Self {
            config,
            world,
            resources
        }
    }

    pub fn register_listener(&mut self, addr: SocketAddr, protocol: Protocol, ctype: ConnType) -> Result<()> {
        let mut poller = self.resources.get_mut::<ListenPoll>().unwrap();
        let tok = poller.get_next();
        let mut listen = ListenerComponent::new(addr, protocol, ctype, tok)?;
        poller.handler.poller.registry().register(&mut listen.listener, tok, Interest::READABLE)?;
        let mut entity = self.world.push((listen,));
        Ok(())
    }

    pub fn setup(&mut self) {
        if let Some(n) = &self.config.net {
            if let Some(l) = &n.listeners {
                let mut success = 0;
                if let Some(plain_telnet) = &l.plain_telnet {
                    if let Ok(tok) = self.register_listener(plain_telnet.clone(), Protocol::Telnet, ConnType::Plain) {
                        success += 1;
                    } else {
                        panic!("Could not open a listening port for plain telnet!");
                    }

                }

                if success == 0 {
                    panic!("Program has no listeners!");
                }

            } else {
                panic!("Program needs listeners to have meaning!");
            }
        } else {
            panic!("Program requires Net configuration to be useful!");
        }
    }

    pub fn run(&mut self) {

        let mut interval = Duration::from_millis(10);

        let mut listen_schedule = Schedule::builder()
            .add_system(poll_listeners_system())
            .add_system(accept_new_connections_system())
            .build();

        let mut socket_io_schedule = Schedule::builder()
            .add_system(poll_connections_system())
            .add_system(process_connection_read_system())
            .add_system(process_connection_newdata_system())
            .add_system(process_connection_outgoing_system())
            .add_system(connection_health_check_system())
            .build();

        let mut delta = interval.clone();

        loop {
            self.resources.insert(Delta(delta));
            let now = Instant::now();
            listen_schedule.execute(&mut self.world, &mut self.resources);
            socket_io_schedule.execute(&mut self.world, &mut self.resources);

            delta = now.elapsed();

            if interval.as_nanos() > delta.as_nanos() {
                let sleep_for = (interval.as_nanos() - delta.as_nanos()) as u64;
                sleep(Duration::from_nanos(sleep_for));
            }
        }
    }
}