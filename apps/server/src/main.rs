#![warn(clippy::all, clippy::pedantic)]

use std::net::SocketAddr;

use actix_web::{App, HttpServer};

mod error;
mod routes;

use error::AppError;
use logger::init_tracing;

#[actix_web::main]
async fn main() -> Result<(), AppError> {
    init_tracing();

    let addr: SocketAddr = "0.0.0.0:8080".parse()?;
    run_server(addr).await
}

async fn run_server(addr: SocketAddr) -> Result<(), AppError> {
    HttpServer::new(|| App::new().configure(routes::routes))
        .bind(addr)?
        .run()
        .await?;

    Ok(())
}
