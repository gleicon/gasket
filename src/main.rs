use actix_web::{middleware, web, App, HttpRequest, HttpResponse, HttpServer, Result};
use clap::{AppSettings, Clap};
use log::info;
use std::env;
use std::sync::Arc;
use url::Url;

mod http_utils;
mod process_manager;
mod tls_utils;

/*
    private_key_path: String,
    certificate_chain_path: String,
    client_ca_path: String,
*/
#[derive(Clap, Debug)]
#[clap(name = "gasket")]
#[clap(setting = AppSettings::ColoredHelp)]
struct GasketOptions {
    /// command to be executed
    #[clap(short = 'e', long = "execute", default_value = "")]
    command: String,

    /// private key cert
    #[clap(short = 'p', long = "private-key")]
    private_key_path: Option<String>,

    /// certificate chain path cert
    #[clap(short = 'c', long = "certificate-chain")]
    certificate_chain_path: Option<String>,

    /// client ca path pem for mTLS
    #[clap(short = 'a', long = "client-ca")]
    client_ca_path: Option<String>,

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
    let port: u16 = env::var("PORT")
        .map(|s| s.parse().unwrap_or(3000))
        .unwrap_or(3000);
    let dest_port = Arc::new(port + 1);

    // proxy settings: always bind to localhost, always proxy to localhost
    let listen_addr = format!("127.0.0.1:{}", port.to_string());
    let gasket_options = GasketOptions::parse();

    std::env::set_var("RUST_LOG", "actix_web=debug,actix_server=debug,gasket=info");
    env_logger::init();

    info!("Gasket --");
    let cmd = gasket_options.command.clone();

    info!("Starting process manager");
    let handle = process_manager::StaticProcessManager::run(cmd).await;
    // mTLS supercedes tls (if mtls is enable -t/--tls is ignored)
    // defaults to http server if none is set
    if gasket_options.mtls_enabled {
        let private_key_path = match gasket_options.private_key_path {
            Some(cert_path) => {
                info!("Private key path: {:?}", cert_path);
                cert_path
            }
            None => "private_key.pem".to_string(),
        };
        let certificate_chain_path = match gasket_options.certificate_chain_path {
            Some(cert_path) => {
                info!("Certificate chain path: {:?}", cert_path);
                cert_path
            }
            None => "certificate_chain.pem".to_string(),
        };

        let client_ca_path = match gasket_options.client_ca_path {
            Some(cert_path) => {
                info!("Client certificate path: {:?}", cert_path);
                cert_path
            }
            None => "client_cert_path.pem".to_string(),
        };

        // mTLS builder
        let builder = match tls_utils::CertificateManager::new_mtls_builder(
            private_key_path,
            certificate_chain_path,
            client_ca_path,
        ) {
            Ok(b) => b,
            Err(e) => {
                info!("mTLS Abort: {}", e);
                handle.close();
                std::process::exit(-1);
                //return Err(e);
            }
        };

        info!("Starting mTLS server");
        let s = HttpServer::new(move || {
            App::new()
                .app_data(web::Data::new(dest_port.clone()))
                .wrap(middleware::Logger::default())
                .default_service(web::route().to(forward))
        })
        .disable_signals()
        .bind_openssl(listen_addr.clone(), builder)
        .unwrap()
        .run()
        .await;
        handle.close();
        return s;
    } else if gasket_options.tls_enabled {
        let private_key_path = match gasket_options.private_key_path {
            Some(cert_path) => {
                info!("Private key path: {:?}", cert_path);
                cert_path
            }
            None => "private_key.pem".to_string(),
        };
        let certificate_chain_path = match gasket_options.certificate_chain_path {
            Some(cert_path) => {
                info!("Certificate chain path: {:?}", cert_path);
                cert_path
            }
            None => "certificate_chain.crt".to_string(),
        };

        // TLS Builder
        let builder = match tls_utils::CertificateManager::new_tls_builder(
            private_key_path,
            certificate_chain_path,
        ) {
            Ok(b) => b,
            Err(e) => {
                info!("TLS Abort: {}", e);
                handle.close();
                std::process::exit(-1);

                //return Err(e);
            }
        };
        info!("Starting TLS server");
        let s = HttpServer::new(move || {
            App::new()
                .app_data(web::Data::new(dest_port.clone()))
                .wrap(middleware::Logger::default())
                .default_service(web::route().to(forward))
        })
        .disable_signals()
        .bind_openssl(listen_addr.clone(), builder)
        .unwrap()
        .run()
        .await;
        handle.close();
        return s;
    }
    info!("Starting server");
    let s = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(dest_port.clone()))
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
    dest_port: web::Data<Arc<u16>>,
) -> Result<HttpResponse, actix_web::Error> {
    info!("request");
    let dest_port = dest_port.as_ref();
    let forward_url = Url::parse(&format!("http://127.0.0.1:{}", dest_port)).unwrap();
    http_utils::Proxy::forward(req, body, &forward_url).await
}
