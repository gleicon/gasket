use actix_web::{middleware, web, App, HttpRequest, HttpResponse, HttpServer, Result};
use clap::{AppSettings, Clap};
use log::info;
use std::net::SocketAddr;

use std::env;
use url::Url;

mod http_utils;
mod process_manager;
mod tls_utils;

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

    /// https(tls)
    #[clap(short = 't', long = "tls")]
    tls_enabled: bool,

    /// https(mTLS)
    #[clap(short = 'm', long = "mtls")]
    mtls_enabled: bool,
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
    let gasket_options = GasketOptions::parse();

    std::env::set_var("RUST_LOG", "actix_web=info,actix_server=info,gasket=info");
    env_logger::init();

    info!("Gasket --");
    let cmd = gasket_options.command.clone();

    info!("Starting process manager");
    let handle = process_manager::StaticProcessManager::run(cmd).await;

    // mTLS supercedes tls (if mtls is enable -t/--tls is ignored)
    if gasket_options.mtls_enabled {
        info!("mTLS enabled");
    } else if gasket_options.tls_enabled {
        info!("TLS enabled");
        match gasket_options.tls_cert {
            Some(cert_path) => info!("TLS Cert path: {:?}", cert_path),
            None => (),
        };
        // load ssl keys
        let builder = tls_utils::CertificateManager::NewTLSBuilder(
            "key.pem".to_string(),
            "cert.pem".to_string(),
        );
        info!("Starting TLS server");

        let s = HttpServer::new(move || {
            App::new()
                .app_data(web::Data::new(dest_port))
                .wrap(middleware::Logger::default())
                .default_service(web::route().to(forward))
        })
        .disable_signals()
        .bind_openssl(listen_addr.clone(), builder.unwrap())
        .unwrap()
        .run()
        .await;
        return s;
    }

    info!("starting server");

    let s = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(dest_port))
            .wrap(middleware::Logger::default())
            .default_service(web::route().to(forward))
    })
    .disable_signals()
    .bind(listen_addr)
    .unwrap()
    .run()
    .await;

    handle.close();
    s
}

async fn forward(
    req: HttpRequest,
    body: web::Bytes,
    dest_port: web::Data<u16>,
) -> Result<HttpResponse, actix_web::Error> {
    info!("request");
    let dest_port = *dest_port.get_ref();
    let destination_addr = SocketAddr::from(([127, 0, 0, 1], dest_port));
    let forward_url = Url::parse(&format!("http://{}", destination_addr)).unwrap();
    http_utils::Proxy::forward(req, body, &forward_url).await
}
