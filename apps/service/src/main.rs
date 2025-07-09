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





fn main() {
    let _ = peer::identity::main();
    let cli = Args::parse();
    if cli.version {
        let authors = crate_authors!().split(':').collect::<Vec<&str>>().join("\", \"");
        println!("Uppe. service {} - Authors: \"{}\"", crate_version!(), authors);
        return;
    }
    let _cfg = config::Config::from_config(cli.config.as_ref()).expect("Failed to fetch config");
    // ZMQ logic removed as part of dead code cleanup
    // Main loop placeholder
    loop {
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}
