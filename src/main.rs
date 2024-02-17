use clap::{AppSettings, Clap};
use log::info;
use std::env;
use std::sync::Arc;

mod http_utils;
mod process_manager;
mod proxy;
mod server;
mod tls_utils;

/*
    certificates (TLS and mTLS)
    - private_key_path: String,
    - certificate_chain_path: String,
    - client_ca_path: String,
*/
#[derive(Clap, Debug, Clone)]
#[clap(name = "gasket")]
#[clap(setting = AppSettings::ColoredHelp)]
pub struct GasketOptions {
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

    /// throttling
    #[clap(short = 'r', long = "throttling")]
    throttling_enabled: bool,

    /// circuit breaker
    #[clap(short = 'b', long = "circuitbreaker")]
    circuitbreaker_enabled: bool,

    /// exponential backoff
    #[clap(short = 'k', long = "backoff")]
    backoff_enabled: bool,
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
    let listen_addr = format!("127.0.0.1:{port}");
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
        let s = server::mtls_server(gasket_options, dest_port, listen_addr).await;
        handle.close();
        return s;
    } else if gasket_options.tls_enabled {
        let s = server::tls_server(gasket_options, dest_port, listen_addr).await;
        handle.close();
        return s;
    }
    let s = server::http_server(gasket_options, dest_port, listen_addr).await;
    handle.close();
    s
}
