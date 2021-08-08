### Certificates
Generate locally signed certificates for testing. Make sure to use the latest openssl binary.
Also make sure that openssl.cnf has the right HOME dir set.

$ sh create_cert.sh 

### testing mTLS with curl
$ export PORT=3000; cargo run -- -m -e "/usr/bin/nc -l 3001" -p certs/certificates/server_certs/server.key.pem -c certs/certificates/server_certs/server.cert.pem -a certs/certificates/mtls/certs/cacert.pe

$ curl --cacert certs/certificates/mtls/certs/cacert.pem --key certs/certificates/client_certs/client.key.pem --cert certs/certificates/client_certs/client.cert.pem https://localhost:3000/test 

