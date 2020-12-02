#[macro_use]
extern crate log;
use actix_web::{web, App, HttpServer, Responder};
use env_logger;

async fn index() -> impl Responder {
    debug!("Received request, preparing response");
    "Hello world!"
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    info!("Starting up!");

    HttpServer::new(|| {
        App::new().service(
            // prefixes all resources and routes attached to it...
            web::scope("/app")
                // ...so this handles requests for `GET /app/index.html`
                .route("/index.html", web::get().to(index)),
        )
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}
