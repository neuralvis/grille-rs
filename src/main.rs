extern crate log;
use std::net::ToSocketAddrs;

use actix_cors::Cors;
use actix_web::client::Client;
use actix_web::{http, middleware, web, App, HttpServer};

use env_logger::{Builder, Target};
use url::Url;

use actix_web_httpauth::middleware::HttpAuthentication;

mod auth;
mod errors;
mod handlers;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "info,actix_web=error");
    // we can't use env_logger::init();
    // because we need to log to stdout instead
    // of stderr, so we can redirect to a file
    // on the commandline
    let mut builder = Builder::from_default_env();
    builder.target(Target::Stdout).init();

    log::debug!("Starting up!");

    // define address:port where the server will listen
    let server_addr = "127.0.0.1";
    let server_port = 8080;

    // define forwarding address and port
    // ideally, we would be taking these from cmdline
    let forward_addr = "127.0.0.1";
    let forward_port: u16 = 8000;

    let forward_url: url::Url = Url::parse(&format!(
        "http://{}",
        (forward_addr, forward_port)
            .to_socket_addrs()
            .unwrap()
            .next()
            .unwrap()
    ))
    .unwrap();

    log::debug!("Forwarding URL: {:?}", forward_url.as_str());

    HttpServer::new(move || {
        // add oauth authentication
        let authenticator = HttpAuthentication::bearer(auth::validator);
        App::new()
            // Authentication validator
            .wrap(authenticator)
            // Cors stuff
            .wrap(
                Cors::default()
                    .allow_any_origin()
                    .allowed_methods(vec!["GET"])
                    .allowed_headers(vec![http::header::AUTHORIZATION, http::header::ACCEPT])
                    .allowed_header(http::header::CONTENT_TYPE)
                    .supports_credentials()
                    .max_age(3600),
            )
            // enable logging
            .wrap(middleware::Logger::default())
            .app_data(web::PayloadConfig::new(2_048_000))
            .data(Client::new())
            .data(forward_url.clone())
            // all default routes to forward() fn
            .default_service(web::route().to(handlers::forward))
    })
    .bind((server_addr, server_port))?
    .run()
    .await
}
