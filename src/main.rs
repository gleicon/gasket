use actix_web::client::Client;
use actix_web::{middleware, web, App, HttpRequest, HttpResponse, HttpServer, Result};
use log::info;
use std::net::SocketAddr;
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
    let id = forward_request.id;
    info!("{}", format!("Request id: {:?}", id.clone()));
    let res = forward_request.send_body(body).await?;
    let mut cb = http_utils::HttpResponseClientBuilder::new(res, id);

    Ok(cb.client_response().await)
}
