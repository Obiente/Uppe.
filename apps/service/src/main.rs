mod peer;
use zmq;

fn main() {
    // Start the ZeroMQ service
    let context = zmq::Context::new();
    let socket = context
        .socket(zmq::REP)
        .unwrap();
    socket
        .bind("tcp://*:5555")
        .unwrap();

    loop {
      
    }
}
