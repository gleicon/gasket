use actix_web::client::Client;
use actix_web::error as actix_error;
use actix_web::http::header::{HeaderMap, HeaderName};
use actix_web::{middleware, web, App, HttpRequest, HttpResponse, HttpServer, Result};
use futures::future::lazy;
use futures::{Future, Stream};
use log::info;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use url::Url;

mod http_utils;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let listen_addr = "127.0.0.1:8443";
    let addr2 = SocketAddr::from(([127, 0, 0, 1], 3000));
    let forward_url = Url::parse(&format!("http://{}", addr2)).unwrap();

    env_logger::init();
    info!("Gasket --");

    HttpServer::new(move || {
        App::new()
            .data(Client::new())
            .data(forward_url.clone())
            .wrap(middleware::Logger::default())
            .default_service(web::route().to(forward))
    })
    .bind(listen_addr)?
    .run()
    .await
}

async fn forward(
    req: HttpRequest,
    body: web::Bytes,
    url: web::Data<Url>,
    client: web::Data<Client>,
) -> Result<HttpResponse, actix_web::Error> {
    let url = url.get_ref();
    let client = client.get_ref();

    let forward_request = http_utils::ForwardRequestClientBuilder::new(req, client, url);

    let res = forward_request
        .client_req
        .send_body(body)
        .await
        .map_err(actix_web::Error::from)?;

    let mut cb = http_utils::HttpResponseClientBuilder::new(res, forward_request.id);

    Ok(cb.client_response().await)
}
