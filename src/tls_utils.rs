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
    ) -> Result<openssl::ssl::SslAcceptorBuilder, std::io::Error> {
        // https://wiki.mozilla.org/Security/Server_Side_TLS#Intermediate_compatibility_.28recommended.29
        let mut builder = SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();
        builder
            .set_min_proto_version(Some(SslVersion::SSL3))
            .unwrap();

        builder.set_session_cache_mode(SslSessionCacheMode::OFF);

        builder
            .set_private_key_file(private_key_path, SslFiletype::PEM)
            .unwrap();
        match builder.set_certificate_chain_file(certificate_chain_path) {
            Ok(_b) => Ok(builder),
            Err(e) => Err(e.into()),
        }
    }

    // "ca/server/client-ssl.key"
    // "ca/server/client-ssl.crt"
    // "ca/ca.crt"
    pub fn new_mtls_builder(
        private_key_path: String,
        certificate_chain_path: String,
        client_ca_path: String,
    ) -> Result<openssl::ssl::SslAcceptorBuilder, std::io::Error> {
        // mtls is tls with specific settings:
        // - a client store so we can store and verify client certificates
        // - SSL_VERIFY_MODE set to verify peers and fail if no cert is given
        // the store could be external or persistent but we opt to create it at start time
        // as containers should be cheap to spin (and we live within them)

        // build client certificate store
        let mut builder =
            match CertificateManager::new_tls_builder(private_key_path, certificate_chain_path) {
                Ok(b) => b,
                Err(e) => return Err(e),
            };

        let ca_cert = fs::read_to_string(client_ca_path)?.into_bytes();
        let client_ca_cert = X509::from_pem(&ca_cert)?;
        let mut x509_client_store_builder = X509StoreBuilder::new()?;
        x509_client_store_builder.add_cert(client_ca_cert)?;
        let local_client_x509_store = x509_client_store_builder.build();
        builder.set_verify_cert_store(local_client_x509_store)?;

        // Verify mode set to fail verifying peer (client) certificate
        let mut mtls_verify_mode = SslVerifyMode::empty();
        mtls_verify_mode.set(SslVerifyMode::PEER, true);
        mtls_verify_mode.set(SslVerifyMode::FAIL_IF_NO_PEER_CERT, true);
        builder.set_verify(mtls_verify_mode);

        Ok(builder)
    }
}
