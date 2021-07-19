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

    std::env::set_var("RUST_LOG", "actix_web=debug,actix_server=debug");
    env_logger::init();
    info!("Gasket --");

    let cmd = gasket_options.command.clone();
    if cmd != "" {
        info!("before spawn");
        process_manager::StaticProcessManager::run(cmd).await;
    };

    match gasket_options.tls_cert {
        Some(cert_path) => info!("TLS Cert path: {:?}", cert_path),
        None => (),
    };
    info!("starting server");

    HttpServer::new(move || {
        App::new()
            // .app_data(forward_url.clone())
            // .app_data(web::Data::new("127.0.0.1.to_string()"))
            .app_data(web::Data::new(dest_port))
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
    dest_port: web::Data<u16>,
    // dest_addr: web::Data<String>,
    // url: web::Data<url::Url>,
) -> Result<HttpResponse, actix_web::Error> {
    info!("request");
    let dest_port = *dest_port.get_ref();
    let destination_addr = SocketAddr::from(([127, 0, 0, 1], dest_port));
    let forward_url = Url::parse(&format!("http://{}", destination_addr)).unwrap();
    http_utils::Proxy::forward(req, body, &forward_url).await
}
