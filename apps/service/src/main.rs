#![feature(inherent_associated_types)]
use std::{env, path};

use clap::{Parser, crate_authors, crate_version};
// use peerup;
mod config;
mod models;

mod pool;
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

#[tokio::main]
async fn main() {
    let cli = Args::parse();
    if cli.version {
        let authors = crate_authors!().split(':').collect::<Vec<&str>>().join("\", \"");
        println!("Uppe. service {} - Authors: \"{}\"", crate_version!(), authors);
        return;
    }
    let cfg = config::Config::from_config(cli.config.as_ref()).expect("Failed to fetch config");
    // ZMQ logic removed as part of dead code cleanup
    let db = libsql::Builder::new_local("libsql.db").build().await.unwrap();

    let manager = pool::LibsqlManager::new(db);
    let pool: pool::LibsqlPool = deadpool::managed::Pool::builder(manager)
        .config(deadpool::managed::PoolConfig::default())
        .build()
        .expect("Failed to build database pool");

    let db_conn = pool.get().await.expect("Failed to get database connection");

    // Check if PeerUP module is enabled in user preferences
    if cfg.preferences.use_peerup_layer {
        // Initialize PeerUP module
        //
    }

    // Main loop placeholder
    loop {
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}
