use serde_derive::Deserialize;
use std::net::SocketAddr;
use std::error::Error;
use std::fs::read_to_string;

#[derive(Deserialize, Debug, Clone)]
pub struct ListenerConfig {
    pub plain_telnet: Option<SocketAddr>,
    pub tls_telnet: Option<SocketAddr>,
    pub plain_websocket: Option<SocketAddr>,
    pub tls_websocket: Option<SocketAddr>,
    pub ssh: Option<SocketAddr>
}

#[derive(Deserialize, Debug, Clone)]
pub struct TlsConfig {
    pub key: String,
    pub pem: String
}

#[derive(Deserialize, Debug, Clone)]
pub struct NetConfig {
    pub listeners: Option<ListenerConfig>,
    pub tls: Option<TlsConfig>
}

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub net: Option<NetConfig>
}

impl Config {
    // Reads a toml file and
    pub fn from_file(file_name: String) -> Result<Self, Box<dyn Error>> {
        let conf_txt = read_to_string(String::from(file_name))?;
        let conf: Self = serde_json::from_str(&conf_txt)?;
        Ok(conf)
    }
}