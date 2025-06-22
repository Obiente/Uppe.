use actix_web::{HttpResponse, Responder, get};

macros_utils::routes! {
    route health_route,
}

/// Health check route
/// This route returns no content, the response status is enough.
#[get("/")]
pub async fn health_route() -> impl Responder {
    HttpResponse::Ok()
}
