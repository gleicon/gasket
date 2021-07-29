### Certificates
Generate locally signed certificates for testing. Make sure to use the latest openssl binary.
Also make sure that openssl.cnf has the right HOME dir set.

$ sh create_cert.sh 

### testing mTLS with curl
curl --cacert certs/mtls/certs/cacert.pem --key certs/client_certs/client.key.pem --cert certs/client_certs/client.cert.pem https://localhost:3000/asdadas
