use std::{
    collections::{HashMap, HashSet},
    vec::Vec,
    io::Write,
    io::Read,
};
use std::sync::Arc;

pub mod codes;
use crate::net::{ConnectionComponent, ProtocolComponent, ProtocolCapabilities};
use bytes::{Buf, BufMut, Bytes, BytesMut};

#[derive(Clone, Debug)]
pub enum TelnetEvent {
    Line(String),
    Command(u8),
    LocalEnable(u8),
    LocalDisable(u8),
    RemoteEnable(u8),
    RemoteDisable(u8),
    LocalHandshake(u8),
    RemoteHandshake(u8),
    SubData(u8, Vec<u8>)
}

#[derive(Clone, Debug)]
pub enum TelnetMessage {
    Data(Vec<u8>),
    IAC(u8),
    Negotiate(u8, u8),
    SubNegotiate(u8, Vec<u8>)
}

impl TelnetMessage {
    pub fn to_vec(&self) -> Vec<u8> {
        match self {
            TelnetMessage::Data(v) => {
                return v.clone()
            },
            TelnetMessage::IAC(code) => {
                return vec![codes::IAC, *code]
            },
            TelnetMessage::Negotiate(code, op) => {
                return vec![codes::IAC, *code, *op]
            },
            TelnetMessage::SubNegotiate(code, data) => {
                let mut arr = Vec::with_capacity(5 + data.len());
                arr.extend_from_slice([codes::IAC, codes::SB, *code].as_ref());
                arr.extend(data);
                arr.extend_from_slice([codes::IAC, codes::SE].as_ref());
                return arr
            }
        }
    }

    pub fn from_bytes(src: &[u8]) -> Option<(TelnetMessage, usize)> {
        if src.is_empty() {
            return None
        }

        if src[0] == codes::IAC {
            match src[1] {
                codes::IAC => {
                    let mut out = Vec::new();
                    out.push(codes::IAC);
                    return Some((TelnetMessage::Data(out), 2))
                },
                codes::WILL | codes::WONT | codes::DO | codes::DONT => {
                    if src.len() > 2 {
                        let answer = TelnetMessage::Negotiate(src[1], src[2]);
                        return Some((answer, 3))
                    } else {
                        // not enough bytes yet.
                        return None
                    }
                },
                codes::SB => {
                    if src.len() > 4 {
                        if let Some(ipos) = src.as_ref().windows(2).position(|b| b[0] == codes::IAC && b[1] == codes::SE) {
                            // Split off any available up to an IAC and stuff it in the sub data buffer.
                            let answer = TelnetMessage::SubNegotiate(src[2], src[3..ipos].to_vec());
                            return Some((answer, ipos+2))
                        } else {
                            return None
                        }
                    } else {
                        // not enough bytes to be a full sequence.
                        return None
                    }
                },
                _ => {
                    // anything else that doesn't match the above is a simple IAC command.
                    return Some((TelnetMessage::IAC(src[1]), 2))
                }
            }
        } else {
            if let Some(ipos) = src.iter().position(|b| b == &codes::IAC) {
                // Split off any available up to an IAC and stuff it in the sub data buffer.
                return Some((TelnetMessage::Data(src[..ipos].to_vec()), ipos))
            } else {
                return Some((TelnetMessage::Data(src.to_vec()), src.len()))
            }
        }
    }
}

#[derive(Default, Clone, Debug)]
pub struct TelnetOptionPerspective {
    pub enabled: bool,
    // Negotiating is true if WE have sent a request.
    pub negotiating: bool
}

#[derive(Default, Clone, Debug)]
pub struct TelnetOptionState {
    pub remote: TelnetOptionPerspective,
    pub local: TelnetOptionPerspective,
}

#[derive(Default, Clone, Debug)]
pub struct TelnetOption {
    pub allow_local: bool,
    pub allow_remote: bool,
    pub start_local: bool,
    pub start_remote: bool,
}

#[derive(Default, Debug, Clone)]
pub struct TelnetHandshakes {
    pub local: HashSet<u8>,
    pub remote: HashSet<u8>,
    pub mtts: HashSet<u8>
}

impl TelnetHandshakes {
    pub fn len(&self) -> usize {
        self.local.len() + self.remote.len() + self.mtts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}


#[derive(Debug)]
pub struct TelnetProtocol {
    pub op_state: HashMap<u8, TelnetOptionState>,
    pub telnet_options: Arc<HashMap<u8, TelnetOption>>,
    pub handshakes_left: TelnetHandshakes,
    pub app_buffer: BytesMut,
    pub mtts_last: Option<String>
    
}

impl TelnetProtocol {

    pub fn new(telnet_options: Arc<HashMap<u8, TelnetOption>>) -> Self {
        let mut op_state: HashMap<u8, TelnetOptionState> = HashMap::with_capacity(telnet_options.len());

        for (key, val) in telnet_options.iter() {
            op_state.insert(*key, TelnetOptionState::default());
        }

        let mut handshakes_left = TelnetHandshakes::default();
        
        Self {
            op_state,
            telnet_options,
            handshakes_left,
            app_buffer: Default::default(),
            mtts_last: None,
        }
    }

    fn send_data(&self, mut writer: &mut impl Write, data: impl AsRef<[u8]>) {
        writer.write_all(data.as_ref());
    }

    pub fn send_sub(&mut self, op: u8, data: impl AsRef<[u8]>, mut writer: &mut impl Write) {
        let mut out = BytesMut::with_capacity(5 + data.as_ref().len());
        out.extend_from_slice(&[codes::IAC, codes::SB, op]);
        out.extend_from_slice(data.as_ref());
        out.extend_from_slice(&[codes::IAC, codes::SE]);
        self.send_data(writer, out);
    }

    pub fn start(&mut self, mut writer: &mut impl Write) {
        let mut out = BytesMut::new();

        for (k, v) in self.telnet_options.iter() {
            if v.start_local {
                out.extend_from_slice(&[codes::IAC, codes::WILL, *k]);
            }
            if v.start_remote {
                out.extend_from_slice(&[codes::IAC, codes::DO, *k]);
            }
        }
        self.send_data(writer, out);
    }

    pub fn process_message(&mut self, msg: TelnetMessage, mut out: &mut Vec<TelnetEvent>,
                           mut writer: &mut impl Write, mut capabilities: &mut ProtocolCapabilities) {
        match msg {
            TelnetMessage::SubNegotiate(op, data) => self.receive_sub(op, data, out, writer, capabilities),
            TelnetMessage::Negotiate(comm, op) => self.receive_negotiate(comm, op, out, writer, capabilities),
            TelnetMessage::IAC(byte) => self.receive_command(byte, out, writer, capabilities),
            TelnetMessage::Data(data) => self.receive_data(data, out)
        }
    }

    fn receive_command(&mut self, command: u8, mut out: &mut Vec<TelnetEvent>, mut writer: &mut impl Write, mut capabilities: &mut ProtocolCapabilities) {

    }

    fn receive_sub(&mut self, op: u8, data: Vec<u8>, mut out: &mut Vec<TelnetEvent>, mut writer: &mut impl Write, mut capabilities: &mut ProtocolCapabilities) {
        if !self.op_state.contains_key(&op) {
            // Only if we can get a handler, do we want to care about this.
            // All other sub-data is ignored.
            return;
        }

        match op {
            codes::NAWS => {
                let _ = self.receive_naws(data, out, writer, capabilities);
            },
            codes::MTTS => {
                let _ = self.receive_mtts(data, out, writer, capabilities);
            }
            _ => {}
        }
    }

    fn receive_mtts(&mut self, data: Vec<u8>, mut out: &mut Vec<TelnetEvent>, mut writer: &mut impl Write, mut capabilities: &mut ProtocolCapabilities) {
        let mut new_data = BytesMut::with_capacity(data.len());
        new_data.extend(data);
        
        if new_data.len() < 2 {
            return
        }

        if self.handshakes_left.mtts.is_empty() {
            return;
        }

        if new_data[0] != 0 {
            return;
        }

        new_data.advance(1);

        if let Ok(s) = String::from_utf8(new_data.to_vec()) {
            let upper = s.trim().to_uppercase();

            if let Some(last) = &self.mtts_last {
                if *last == upper {
                    // We're not going to learn anything else from this client.
                    self.handshakes_left.mtts.clear();
                    return;
                }
            }

            let hs1: u8 = 0;
            let hs2: u8 = 0;
            let hs3: u8 = 0;
            self.mtts_last = Some(upper.clone());
            if self.handshakes_left.mtts.contains(&hs1) {
                self.receive_mtts_0(upper, out, writer, capabilities);
                self.handshakes_left.mtts.remove(&hs1);
                self.send_sub(codes::MTTS, &[1], writer);
                return;
            } else if self.handshakes_left.mtts.contains(&hs2) {
                self.receive_mtts_1(upper, out, writer, capabilities);
                self.handshakes_left.mtts.remove(&hs2);
                self.send_sub(codes::MTTS, &[1], writer);
                return;
            } else if self.handshakes_left.mtts.contains(&hs3) {
                self.receive_mtts_2(upper, out, writer, capabilities);
                self.handshakes_left.mtts.remove(&hs3);
                return;
            }

        }
    }

    fn receive_mtts_0(&mut self, data: String, mut out: &mut Vec<TelnetEvent>, mut writer: &mut impl Write, mut capabilities: &mut ProtocolCapabilities) {
        // The first mtts receives the name of the client.
        // version might also be in here as a second word.
        if data.contains(" ") {
            let results: Vec<&str> = data.splitn(1, " ").collect();
            capabilities.client_name = String::from(results[0]);
            capabilities.client_version = String::from(results[1]);
        } else {
            capabilities.client_name = data;
        }

        // Now that the name and version (may be UNKNOWN) are set... we can deduce capabilities.
        let mut extra_check = false;
        match capabilities.client_name.as_str() {
            "ATLANTIS" => {
                capabilities.xterm256 = true;
                capabilities.ansi = true;
            },
            "CMUD" => {
                capabilities.xterm256 = true;
                capabilities.ansi = true;
            },
            "KILDCLIENT" => {
                capabilities.xterm256 = true;
                capabilities.ansi = true;
            },
            "MUDLET" => {
                capabilities.xterm256 = true;
                capabilities.ansi = true;
            },
            "MUSHCLIENT" => {
                capabilities.xterm256 = true;
                capabilities.ansi = true;
            },
            "PUTTY" => {
                capabilities.xterm256 = true;
                capabilities.ansi = true;
            },
            "BEIP" => {
                capabilities.xterm256 = true;
                capabilities.ansi = true;
            },
            "POTATO" => {
                capabilities.xterm256 = true;
                capabilities.ansi = true;
            },
            "TINYFUGUE" => {
                capabilities.xterm256 = true;
                capabilities.ansi = true;
            }
            _ => {
                extra_check = true;
            }
        }
        if extra_check {
            if capabilities.client_name.starts_with("XTERM") || capabilities.client_name.ends_with("-256COLOR") {
                capabilities.xterm256 = true;
                capabilities.ansi = true;
            }
        }
    }

    fn receive_mtts_1(&mut self, data: String, mut out: &mut Vec<TelnetEvent>, mut writer: &mut impl Write, mut capabilities: &mut ProtocolCapabilities) {
        if data.starts_with("XTERM") || data.ends_with("-256COLOR") {
            capabilities.xterm256 = true;
            capabilities.ansi = true;
        }
    }

    fn receive_mtts_2(&mut self, data: String, mut out: &mut Vec<TelnetEvent>, mut writer: &mut impl Write, mut capabilities: &mut ProtocolCapabilities) {
        if !data.starts_with("MTTS ") {
            return;
        }
        let results: Vec<&str> = data.splitn(2, " ").collect();
        let value = String::from(results[1]);
        let mtts: usize = value.parse().unwrap_or(0);
        if mtts == 0 {
            return;
        }
        if (1 & mtts) == 1 {
            capabilities.ansi = true;
        }
        if (2 & mtts) == 2 {
            capabilities.vt100 = true;
        }
        if (4 & mtts) == 4 {
            capabilities.utf8 = true;
        }
        if (8 & mtts) == 8 {
            capabilities.xterm256 = true;
        }
        if (16 & mtts) == 16 {
            capabilities.mouse_tracking = true;
        }
        if (32 & mtts) == 32 {
            capabilities.osc_color_palette = true;
        }
        if (64 & mtts) == 64 {
            capabilities.screen_reader = true;
        }
        if (128 & mtts) == 128 {
            capabilities.proxy = true;
        }
        if (256 & mtts) == 256 {
            capabilities.truecolor = true;
        }
        if (512 & mtts) == 512 {
            capabilities.mnes = true;
        }

    }
    
    fn receive_naws(&mut self, mut data: Vec<u8>, mut out: &mut Vec<TelnetEvent>, mut writer: &mut impl Write, mut capabilities: &mut ProtocolCapabilities) {
        let mut new_data = BytesMut::with_capacity(data.len());
        new_data.extend(data);
        if new_data.len() >= 4 {
            capabilities.width = new_data.get_u16();
            capabilities.height = new_data.get_u16();
        }
    }
    
    fn receive_data(&mut self, data: Vec<u8>, mut out: &mut Vec<TelnetEvent>) {
        self.app_buffer.extend(data);
        while let Some(ipos) = self.app_buffer.as_ref().iter().position(|b| b == &codes::LF) {
            let cmd = self.app_buffer.split_to(ipos);
            if let Ok(s) = String::from_utf8(cmd.to_vec()) {
                out.push(TelnetEvent::Line(s.trim().to_string()));
            }
            self.app_buffer.advance(1);
        }
    }

    fn receive_negotiate(&mut self, command: u8, op: u8, mut out: &mut Vec<TelnetEvent>, mut writer: &mut impl Write, mut capabilities: &mut ProtocolCapabilities) {
        let mut handshake: u8 = 0;
        let mut enable_local = false;
        let mut disable_local = false;
        let mut enable_remote = false;
        let mut disable_remote = false;
        let mut handshake_remote: u8 = 0;
        let mut handshake_local: u8 = 0;
        let mut respond: u8 = 0;

        if let Some(state) = self.op_state.get_mut(&op) {
            // We DO have a handler for this option... that means we support it!

            match command {
                codes::WILL => {
                    // The remote host has sent a WILL. They either want to Locally-Enable op, or are
                    // doing so at our request.
                    if !state.remote.enabled {
                        if state.remote.negotiating {
                            state.remote.negotiating = false;
                        }
                        else {
                            respond = codes::DO;
                        }
                        handshake = op;
                        handshake_remote = op;
                        enable_remote = true;
                        state.remote.enabled = true;
                    }
                },
                codes::WONT => {
                    // The client has refused an option we wanted to enable. Alternatively, it has
                    // disabled an option that was on.
                    if state.remote.negotiating {
                        handshake = op;
                        handshake_remote = op;
                    }
                    state.remote.negotiating = false;
                    if state.remote.enabled {
                        disable_remote = true;
                        state.remote.enabled = false;
                    }
                },
                codes::DO => {
                    // The client wants the Server to enable Option, or they are acknowledging our
                    // desire to do so.
                    if !state.local.enabled {
                        if state.local.negotiating {
                            state.local.negotiating = false;
                        }
                        else {
                            respond = codes::WILL;
                        }
                        handshake = op;
                        handshake_local = op;
                        enable_local = true;
                        state.local.enabled = true;
                    }
                },
                codes::DONT => {
                    // The client wants the server to disable Option, or are they are refusing our
                    // desire to do so.
                    if state.local.negotiating {
                        handshake = op;
                        handshake_local = op;
                    }
                    state.local.negotiating = false;
                    if state.local.enabled {
                        disable_local = true;
                        state.local.enabled = false
                    }
                },
                _ => {
                    // This cannot actually happen.
                }
            }
        } else {
            // We do not have a handler for this option, whatever it is... do not support.
            respond = match command {
                codes::WILL => codes::DONT,
                codes::DO => codes::WONT,
                _ => 0
            };
        }

        if respond > 0 {
            let _ = self.send_data(writer,&[codes::IAC, respond, op]);
        }
        if handshake_local > 0 {
            out.push(TelnetEvent::LocalHandshake(op));
        }
        if handshake_remote > 0 {
            out.push(TelnetEvent::RemoteHandshake(op));
        }
        if enable_local {
            self.enable_local(op, out, writer, capabilities);
        }
        if disable_local {
            self.disable_local(op, out, writer, capabilities);
        }
        if enable_remote {
            self.enable_remote(op, out, writer, capabilities);
        }
        if disable_remote {
            self.disable_remote(op, out, writer, capabilities);
        }
    }

    fn enable_local(&mut self, op: u8, mut out: &mut Vec<TelnetEvent>, mut writer: &mut impl Write,
                    mut capabilities: &mut ProtocolCapabilities) {
        match op {
            codes::SGA => {
                capabilities.sga = true;
            },
            codes::MXP => {
                capabilities.mxp = true;
                self.send_sub(codes::MXP, &[], writer);
            }
            _ => {

            }
        }
    }

    fn enable_remote(&mut self, op: u8, mut out: &mut Vec<TelnetEvent>, mut writer: &mut impl Write,
                    mut capabilities: &mut ProtocolCapabilities) {
        match op {
            codes::NAWS => capabilities.naws = true,
            codes::MTTS => {
                self.handshakes_left.mtts.insert(0);
                self.handshakes_left.mtts.insert(1);
                self.handshakes_left.mtts.insert(2);
                self.send_sub(codes::MTTS, &[1], writer);
            },
            codes::LINEMODE => capabilities.linemode = true,
            _ => {
                // Whatever this option is.. well, whatever.
            }
        }
    }

    fn disable_remote(&mut self, op: u8, mut out: &mut Vec<TelnetEvent>, mut writer: &mut impl Write,
                     mut capabilities: &mut ProtocolCapabilities) {
        match op {
            codes::NAWS => {
                capabilities.naws = false;
                capabilities.width = 78;
                capabilities.height = 24;
            }
            codes::MTTS => {
                capabilities.mtts = false;
                self.handshakes_left.mtts.clear();
            },
            codes::LINEMODE => capabilities.linemode = false,
            _ => {
                // Whatever this option is.. well, whatever.
            }
        }
    }

    fn disable_local(&mut self, op: u8, mut out: &mut Vec<TelnetEvent>, mut writer: &mut impl Write,
                      mut capabilities: &mut ProtocolCapabilities) {
        match op {
            codes::SGA => {
                capabilities.sga = false;
            },
            codes::MXP => {
                capabilities.mxp = false;
            }
            _ => {

            }
        }
    }
}