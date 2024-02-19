use actix_web::{middleware, web, App, HttpServer};
use log::info;
use signal_hook_tokio::Handle;
use std::env;
use std::sync::Arc;
use std::sync::Mutex;

use crate::GasketOptions;

pub async fn start_server(go: GasketOptions, handle: Handle) -> std::io::Result<()> {
    // PORT will always be a pair like:
    // Origin port is 3000, destination port will be 3001
    // Gasket increments the port by one based on the PORT env var
    // or defaults to port 3000
    let port: u16 = env::var("PORT")
        .map(|s| s.parse().unwrap_or(3000))
        .unwrap_or(3000);
    let dest_port = Arc::new(port + 1);
    info!("PORT: {port}, backend PORT: {dest_port}");
    if go.backoff_enabled {
        info!("Backoff enabled");
    }
    // proxy settings: always bind to localhost, always proxy to localhost
    let listen_addr = format!("127.0.0.1:{port}");
    // mTLS supercedes tls (if mtls is enable -t/--tls is ignored)
    // defaults to http server if none is set
    if go.mtls_enabled {
        let s = mtls_server(go, dest_port, listen_addr).await;
        handle.close();
        return s;
    } else if go.tls_enabled {
        let s = tls_server(go, dest_port, listen_addr).await;
        handle.close();
        return s;
    }
    http_server(go, dest_port, listen_addr).await
}

pub async fn mtls_server(
    gasket_options: crate::GasketOptions,
    dest_port: Arc<u16>,
    listen_addr: String,
) -> std::result::Result<(), std::io::Error> {
    let private_key_path = match gasket_options.private_key_path.clone() {
        Some(cert_path) => {
            info!("Private key path: {:?}", cert_path);
            cert_path
        }
        None => "private_key.pem".to_string(),
    };
    let certificate_chain_path = match gasket_options.certificate_chain_path.clone() {
        Some(cert_path) => {
            info!("Certificate chain path: {:?}", cert_path);
            cert_path
        }
        None => "certificate_chain.pem".to_string(),
    };

    let client_ca_path = match gasket_options.client_ca_path.clone() {
        Some(cert_path) => {
            info!("Client certificate path: {:?}", cert_path);
            cert_path
        }
        None => "client_cert_path.pem".to_string(),
    };

    // mTLS builder
    let builder = match crate::tls_utils::CertificateManager::new_mtls_builder(
        private_key_path,
        certificate_chain_path,
        client_ca_path,
    ) {
        Ok(b) => b,
        Err(e) => {
            info!("mTLS Abort: {}", e);
            std::process::exit(-1);
        }
    };

    info!("Starting mTLS server");
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(dest_port.clone()))
            .app_data(web::Data::new(Arc::new(Mutex::new(gasket_options.clone()))))
            .wrap(middleware::Logger::default())
            .default_service(web::route().to(crate::proxy::forward))
    })
    .disable_signals()
    .workers(12)
    .bind_openssl(listen_addr.clone(), builder)
    .unwrap()
    .run()
    .await
}

pub async fn tls_server(
    gasket_options: crate::GasketOptions,
    dest_port: Arc<u16>,
    listen_addr: String,
) -> std::result::Result<(), std::io::Error> {
    let private_key_path = match gasket_options.private_key_path.clone() {
        Some(cert_path) => {
            info!("Private key path: {:?}", cert_path);
            cert_path
        }
        None => "private_key.pem".to_string(),
    };
    let certificate_chain_path = match gasket_options.certificate_chain_path.clone() {
        Some(cert_path) => {
            info!("Certificate chain path: {:?}", cert_path);
            cert_path
        }
        None => "certificate_chain.crt".to_string(),
    };

    // TLS Builder
    let builder = match crate::tls_utils::CertificateManager::new_tls_builder(
        private_key_path,
        certificate_chain_path,
    ) {
        Ok(b) => b,
        Err(e) => {
            info!("TLS Abort: {}", e);
            std::process::exit(-1);
        }
    };

    info!("Starting TLS server");
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(dest_port.clone()))
            .app_data(web::Data::new(Arc::new(Mutex::new(gasket_options.clone()))))
            .wrap(middleware::Logger::default())
            .default_service(web::route().to(crate::proxy::forward))
    })
    .disable_signals()
    .workers(12)
    .bind_openssl(listen_addr.clone(), builder)
    .unwrap()
    .run()
    .await
}

pub async fn http_server(
    gasket_options: crate::GasketOptions,
    dest_port: Arc<u16>,
    listen_addr: String,
) -> std::result::Result<(), std::io::Error> {
    info!("Starting HTTP server");
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(dest_port.clone()))
            .app_data(web::Data::new(Arc::new(Mutex::new(gasket_options.clone()))))
            .wrap(middleware::Logger::default())
            .default_service(web::route().to(crate::proxy::forward))
    })
    .disable_signals()
    .workers(12)
    .bind(listen_addr)
    .unwrap()
    .run()
    .await
}
