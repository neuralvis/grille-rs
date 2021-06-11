use actix_web::client::Client;
use actix_web::{web, Error, HttpRequest, HttpResponse};

use url::Url;

// this is our handler
pub async fn forward(
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
