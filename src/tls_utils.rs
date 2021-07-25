use openssl::ssl::{
    SslAcceptor, SslFiletype, SslMethod, SslSessionCacheMode, SslVerifyMode, SslVersion,
};
use openssl::x509::store::X509StoreBuilder;
use openssl::x509::X509;

use std::fs;
pub struct CertificateManager {}

impl CertificateManager {
    // private key and pem file (cert chain)
    pub fn new_tls_builder(
        private_key_path: String,
        certificate_chain_path: String,
    ) -> Result<openssl::ssl::SslAcceptorBuilder, String> {
        let mut builder = SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();
        builder
            .set_private_key_file(private_key_path, SslFiletype::PEM)
            .unwrap();
        builder
            .set_certificate_chain_file(certificate_chain_path)
            .unwrap();
        Ok(builder)
    }

    // "ca/server/client-ssl.key"
    // "ca/server/client-ssl.crt"
    // "ca/ca.crt"
    pub fn new_mtls_builder(
        private_key_path: String,
        certificate_chain_path: String,
        client_ca_path: String,
    ) -> Result<openssl::ssl::SslAcceptorBuilder, std::io::Error> {
        let mut builder = SslAcceptor::mozilla_modern(SslMethod::tls())?;
        builder.set_min_proto_version(Some(SslVersion::SSL3))?;
        builder.set_session_cache_mode(SslSessionCacheMode::OFF);

        builder.set_private_key_file(private_key_path, SslFiletype::PEM)?;
        builder.set_certificate_chain_file(certificate_chain_path)?;

        let ca_cert = fs::read_to_string(client_ca_path)?.into_bytes();
        let client_ca_cert = X509::from_pem(&ca_cert)?;
        let mut x509_client_store_builder = X509StoreBuilder::new()?;
        x509_client_store_builder.add_cert(client_ca_cert)?;
        let local_client_x509_store = x509_client_store_builder.build();
        builder.set_verify_cert_store(local_client_x509_store)?;

        // Enable mTLS, fail verifying peer (client) certificate
        let mut mtls_verify_mode = SslVerifyMode::empty();
        mtls_verify_mode.set(SslVerifyMode::PEER, true);
        mtls_verify_mode.set(SslVerifyMode::FAIL_IF_NO_PEER_CERT, true);
        builder.set_verify(mtls_verify_mode);

        Ok(builder)
    }
}
