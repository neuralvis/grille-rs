extern crate log;
use std::net::ToSocketAddrs;

use actix_web::client::Client;
use actix_web::{
    http, middleware, web, App, Error, HttpRequest, HttpResponse, HttpServer
};

use env_logger;
use url::Url;

/* TODO
 * - Incorporate clap - commandline parser
*/

// this is our handler
async fn forward(
    req: HttpRequest,
    client: web::Data<Client>,
    url: web::Data<Url>,
) -> Result<HttpResponse, Error> {
    log::debug!("REQ: {:?}", req);

    log::debug!("Received request, preparing a forwarding url");
    let mut new_url: url::Url = url.get_ref().clone();
    new_url.set_path(req.uri().path());
    new_url.set_query(req.uri().query());

    log::debug!("Preparing a forward request with url {}", new_url.as_str());
    let req = client
        .get(new_url.as_str())
        // set the headers to allow CORS
        .header(http::header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        // set preflight response to indicate permitted headers
        .header(http::header::ACCESS_CONTROL_ALLOW_HEADERS, "content-type")
        .header(http::header::CONTENT_TYPE, "application/json")
        // only allow GET requests to pass through
        .header(http::header::ACCESS_CONTROL_ALLOW_METHODS, "GET")
        .no_decompress();

    log::debug!("Soliciting response:");
    let mut res = req.send().await.map_err(Error::from)?;
    let mut client_resp = HttpResponse::build(res.status());
    log::debug!("Disconnecting from server");
    // Remove `Connection` as per
    // https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Connection#Directives
    for (header_name, header_value) in res.headers().iter() {
        client_resp.header(header_name.clone(), header_value.clone());
    }

    // println!("Response: {}", res.body().await?);

    Ok(client_resp
        .content_type("application/json")
        .body(res.body().limit(1_024_000).await?))
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
        App::new()
            // enable logging
            .wrap(middleware::Logger::default())
            .app_data(web::PayloadConfig::new(1_024__000))
            .data(Client::new())
            .data(forward_url.clone())
            // all default routes to forward() fn
            .default_service(web::route().to(forward))
    })
    .bind((server_addr, server_port))?
    .run()
    .await
}
