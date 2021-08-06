use actix_web::{middleware, web, App, HttpServer};
use log::info;
use std::sync::{Arc, Mutex};

pub async fn mtls_server(
    gasket_options: crate::GasketOptions,
    dest_port: Arc<u16>,
    listen_addr: String,
) -> std::result::Result<(), std::io::Error> {
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
    let s = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(dest_port.clone()))
            .wrap(middleware::Logger::default())
            .default_service(web::route().to(crate::proxy::forward))
    })
    .disable_signals()
    .bind_openssl(listen_addr.clone(), builder)
    .unwrap()
    .run()
    .await;
    return s;
}

pub async fn tls_server(
    gasket_options: crate::GasketOptions,
    dest_port: Arc<u16>,
    listen_addr: String,
) -> std::result::Result<(), std::io::Error> {
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
    let s = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(dest_port.clone()))
            .wrap(middleware::Logger::default())
            .default_service(web::route().to(crate::proxy::forward))
    })
    .disable_signals()
    .bind_openssl(listen_addr.clone(), builder)
    .unwrap()
    .run()
    .await;
    return s;
}

pub async fn http_server(
    gasket_options: crate::GasketOptions,
    dest_port: Arc<u16>,
    listen_addr: String,
) -> std::result::Result<(), std::io::Error> {
    let sp = Arc::new(Mutex::new(
        crate::stability_patterns::StabilityPatterns::new(),
    ));
    info!("Starting HTTP server");
    let s = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(dest_port.clone()))
            .app_data(web::Data::new(sp.clone()))
            .wrap(middleware::Logger::default())
            .default_service(web::route().to(crate::proxy::forward))
    })
    .disable_signals()
    .workers(12)
    .bind(listen_addr)
    .unwrap()
    .run()
    .await;

    s
}
