use actix_web::client::Client;
use actix_web::{middleware, web, App, HttpRequest, HttpResponse, HttpServer, Result};
use clap::{AppSettings, Clap};
use log::info;
use std::net::SocketAddr;

use std::env;
use url::Url;

mod http_utils;
mod process_manager;

#[derive(Clap, Debug)]
#[clap(name = "gasket")]
#[clap(setting = AppSettings::ColoredHelp)]
struct GasketOptions {
    /// command to be executed
    #[clap(short = 'e', long = "execute", default_value = "")]
    command: String,

    /// tls cert path
    #[clap(short = 'c', long = "cert")]
    tls_cert: Option<String>,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // PORT will always be a pair like:
    // Origin port is 3000, destination port will be 3001
    // Gasket increments the port by one based on the PORT env var
    // or defaults to port 3000
    let port = env::var("PORT")
        .map(|s| s.parse().unwrap_or(3000))
        .unwrap_or(3000);
    let dest_port = port + 1;

    // proxy settings
    // always bind to localhost, always proxy to localhost
    let listen_addr = format!("127.0.0.1:{}", port.to_string());
    let destination_addr = SocketAddr::from(([127, 0, 0, 1], dest_port));
    let forward_url = Url::parse(&format!("http://{}", destination_addr)).unwrap();
    let gasket_options = GasketOptions::parse();

    std::env::set_var("RUST_LOG", "actix_web=info,actix_server=trace");
    env_logger::init();
    info!("Gasket --");

    let cmd = gasket_options.command.clone();
    if cmd != "" {
        let mut pm = process_manager::StaticProcessManager::new().await;
        info!("before spawn");
        pm.spawn_process(cmd).await;
    };

    match gasket_options.tls_cert {
        Some(cert_path) => info!("TLS Cert path: {:?}", cert_path),
        None => (),
    };

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
