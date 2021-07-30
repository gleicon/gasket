export OPENSSL_BIN="/usr/local//Cellar/openssl@1.1/1.1.1k/bin/openssl"
export OPENSSL_CONF="./openssl.cnf"
export CLIENT_EXT="./client_ext.conf"
export SERVER_EXT="./server_ext.conf"
export OPENSSL_SUBJ='/CN=localhoster/O=7co/C=BR/L=SP/ST=SaoPaulo/OU=HR'
export OPENSSL_SERVER_SUBJ='/CN=localhost/O=7co/C=BR/L=SP/ST=SaoPaulo/OU=HR'


echo "Creating test pki structure and keys"

mkdir certificates
mkdir certificates/mtls
mkdir certificates/mtls/newcerts
mkdir certificates/mtls/certs 
mkdir certificates/mtls/private
mkdir certificates/client_certs
mkdir certificates/server_certs

echo 01 > certificates/mtls/serial
touch certificates/mtls/index.txt

$OPENSSL_BIN genrsa -out certificates/mtls/private/cakey.pem 4096
$OPENSSL_BIN req -new -x509 -days 3650 -subj $OPENSSL_SUBJ  -key certificates/mtls/private/cakey.pem -out certificates/mtls/certs/cacert.pem
$OPENSSL_BIN x509 -in certificates/mtls/certs/cacert.pem -out certificates/mtls/certs/cacert.pem -outform PEM

echo "Creating client certificates"
$OPENSSL_BIN genrsa -out certificates/client_certs/client.key.pem 4096
$OPENSSL_BIN req -new -subj $OPENSSL_SUBJ -key certificates/client_certs/client.key.pem -out certificates/client_certs/client.csr
$OPENSSL_BIN ca -config $OPENSSL_CONF -subj $OPENSSL_SUBJ -extfile $CLIENT_EXT -days 1650 -notext -batch -in certificates/client_certs/client.csr -out certificates/client_certs/client.cert.pem

echo "Creating server certificates"
$OPENSSL_BIN genrsa -out certificates/server_certs/server.key.pem 4096
$OPENSSL_BIN req -new -subj $OPENSSL_SERVER_SUBJ -key certificates/server_certs/server.key.pem -out certificates/server_certs/server.csr
$OPENSSL_BIN ca -config $OPENSSL_CONF -subj $OPENSSL_SERVER_SUBJ -extfile $SERVER_EXT -days 1650 -notext -batch -in certificates/server_certs/server.csr -out certificates/server_certs/server.cert.pem
