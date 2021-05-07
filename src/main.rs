use mudcrab::engine::Engine;
use mudcrab::config::Config;
use serde::{Deserialize, Serialize};
use serde_json::Result;

fn main() {
    let conf = Config::from_file(String::from("config.json")).unwrap();

    let mut eng = Engine::new(conf);
    eng.setup();
    eng.run();
    println!("Hello, Config: {:?}", eng.config);
}
