mod peer;
use zmq;
use std::env;
fn get_env_var<T: std::str::FromStr>(name: &str, default: T) -> T {
    match env::var(name) {
        Ok(val) => val.parse().unwrap_or(default),
        Err(_) => default,
    }
}

fn main() {
    let bind: String = get_env_var("BIND", "*".to_string());
    let port: u16 = get_env_var("PORT", 5555);
    let context = zmq::Context::new();
    let socket = context
        .socket(zmq::REP)
        .unwrap();
    socket
        .bind(&format!("tcp://{}:{}", bind, port))
        .unwrap();

        // peer::announcer::(&socket);
        // peer::announcer::(&socket);
    loop {
      
    }
}
