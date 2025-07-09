use std::{env, path};

use clap::{Parser, crate_authors, crate_version};

mod config;
mod peer;
#[derive(Parser, Debug)]
#[command(about = "Uppe. - Monitoring that doesn't let you down!", long_about = None)]
struct Args {
    #[arg(short = 'V', long)]
    /// Print version
    version: bool,

    #[arg(long)]
    /// Path to specific config file
    config: Option<path::PathBuf>,
}

struct ZmqConn {
    pub ctx: zmq::Context,
    pub sock: zmq::Socket,
}

impl ZmqConn {
    pub fn new(cfg: &config::Config) -> Self {
        let bind: String = env::var("BIND").unwrap_or_else(|_: env::VarError| {
            // Debug print env err
            String::clone(&cfg.zeromq.bind)
        });

        let port: u16 = match env::var("PORT") {
            Ok(raw_val) => {
                if let Ok(parsed_val) = raw_val.parse::<u16>() {
                    parsed_val
                } else {
                    // Debug print bad env data (could not parse)
                    cfg.zeromq.port
                }
            },
            Err(_) => {
                // Debug print env err
                cfg.zeromq.port
            },
        };

        let context = zmq::Context::new();
        let socket = context.socket(zmq::REP).unwrap();
        socket.bind(&format!("tcp://{bind}:{port}")).unwrap();

        Self { ctx: context, sock: socket }
    }

    pub fn poll(&self) {}
}

fn main() {
    let _ = peer::identity::main();
    let cli = Args::parse();
    if cli.version {
        let authors = crate_authors!().split(':').collect::<Vec<&str>>().join("\", \"");
        println!("Uppe. service {} - Authors: \"{}\"", crate_version!(), authors);
        return;
    }
    let cfg = config::Config::from_config(cli.config.as_ref()).expect("Failed to fetch config");
    let zmq_conn = ZmqConn::new(&cfg);

    loop {
        zmq_conn.poll();
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}
