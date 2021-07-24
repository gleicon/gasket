use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};

pub struct CertificateManager {}

impl CertificateManager {
    pub fn NewTLSBuilder(
        key_path: String,
        pem_path: String,
    ) -> Result<openssl::ssl::SslAcceptorBuilder, String> {
        let mut builder = SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();
        builder
            .set_private_key_file(key_path, SslFiletype::PEM)
            .unwrap();
        builder.set_certificate_chain_file(pem_path).unwrap();
        Ok(builder)
    }
}
