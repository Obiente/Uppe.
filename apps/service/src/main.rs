mod peer;
use zmq;
use std::env;
fn get_env_var<T: std::str::FromStr>(name: &str, default: T) -> T {
    match env::var(name) {
        Ok(val) => val.parse().unwrap_or(default),
        Err(_) => default,
    }
}
const BIND: &str = get_env_var("BIND", "*");
const PORT: u16 = get_env_var::<u16>("PORT", 5555);
fn main() {
    // Start the ZeroMQ service
    let context = zmq::Context::new();
    let socket = context
        .socket(zmq::REP)
        .unwrap();
    socket
        .bind(&format!("tcp://{}:{}", BIND, PORT))
        .unwrap();

        peer::manager::find_peer_announcer(&socket);
    loop {
      
    }
}
