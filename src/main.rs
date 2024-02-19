use clap::{AppSettings, Clap};
use log::info;

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

    /// exponential backoff
    #[clap(short = 'k', long = "backoff")]
    backoff_enabled: bool,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let gasket_options = GasketOptions::parse();

    std::env::set_var("RUST_LOG", "actix_web=debug,actix_server=debug,gasket=info");
    env_logger::init();

    info!("Gasket --");
    let cmd = gasket_options.command.clone();

    info!("Starting process manager");
    let handle = process_manager::StaticProcessManager::run(cmd).await;
    server::start_server(gasket_options, handle).await
}
