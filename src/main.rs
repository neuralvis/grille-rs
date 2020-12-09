extern crate log;
use std::net::ToSocketAddrs;

use actix_cors::Cors;
use actix_web::client::Client;
use actix_web::{
    dev::ServiceRequest, http, middleware, web, App, Error, HttpRequest, HttpResponse, HttpServer,
};

use env_logger;
use url::Url;

use actix_web_httpauth::extractors::bearer::{BearerAuth, Config};
use actix_web_httpauth::extractors::AuthenticationError;
use actix_web_httpauth::middleware::HttpAuthentication;

mod auth;
mod errors;

async fn validator(req: ServiceRequest, credentials: BearerAuth) -> Result<ServiceRequest, Error> {
    let config = req
        .app_data::<Config>()
        .map(|data| data.clone())
        .unwrap_or_else(Default::default);
    match auth::validate_token(credentials.token()).await {
        Ok(res) => {
            if res == true {
                Ok(req)
            } else {
                Err(AuthenticationError::from(config).into())
            }
        }
        Err(_) => Err(AuthenticationError::from(config).into()),
    }
}

/* TODO
 * - Incorporate clap - commandline parser
 * - Add token-base authentication
*/

// this is our handler
async fn forward(
    req: HttpRequest,
    client: web::Data<Client>,
    url: web::Data<Url>,
) -> Result<HttpResponse, Error> {
    log::info!("{}: {:?}", req.method().as_str(), req.uri());

    log::debug!("Received request, preparing a forwarding url");
    let mut new_url: url::Url = url.get_ref().clone();
    new_url.set_path(req.uri().path());
    new_url.set_query(req.uri().query());

    log::debug!("Preparing a forward request with url {}", new_url.as_str());
    let req = client.get(new_url.as_str()).no_decompress();

    log::debug!("Soliciting response:");
    let mut res = req.send().await.map_err(Error::from)?;
    let mut client_resp = HttpResponse::build(res.status());
    log::debug!("Disconnecting from server");
    // copy headers
    for (header_name, header_value) in res.headers().iter() {
        client_resp.header(header_name.clone(), header_value.clone());
    }

    // println!("Response: {}", res.body().await?);

    Ok(client_resp
        .content_type("application/json")
        .body(res.body().limit(2_048_000).await?))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "info,actix_web=error");
    env_logger::init();
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
        let authenticator = HttpAuthentication::bearer(validator);
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
            .default_service(web::route().to(forward))
    })
    .bind((server_addr, server_port))?
    .run()
    .await
}
