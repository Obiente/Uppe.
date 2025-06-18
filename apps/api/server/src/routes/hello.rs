use actix_web::{HttpResponse, Responder, get};

macros_utils::routes! {
    route route_hello,
}

#[get("/")]
pub async fn route_hello() -> impl Responder {
    HttpResponse::Ok().body("Hello, world!")
}
