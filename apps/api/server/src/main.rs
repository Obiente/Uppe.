use std::io::Error as IoError;

use actix_web::web::get;
use actix_web::{App, HttpResponse, HttpServer};
use logger::init_tracing;
use thiserror::Error as ThisError;

mod routes;

#[derive(Debug, ThisError)]
pub enum AppError {
    #[error("{0:#}")]
    Io(#[from] IoError),
}

#[actix_web::main]
async fn main() -> Result<(), AppError> {
    init_tracing();

    let _ = HttpServer::new(|| {
        App::new()
            .route("/", get().to(HttpResponse::Ok))
            .configure(routes::routes)
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await;

    Ok(())
}
