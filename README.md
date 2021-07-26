# Gasket 

## Gasket - container proxy and PID1 manager

### What is it ?

Gasket is both a proxy which can terminate TLS and mTLS connections and a PID1 manager.

### How ?

- Use it as the container ENTRYPOINT to wrap any 12 factor service or application.
- Gasket behaves as a PID1 manager: after starting it will spin up the https endpoint, translate the received environment variables and spawn the service
- Gasket also does basic signal management and process supervision - if the process dies it will restart it up until a configurable limit.
- Gasket is meant to be used with 12 Factor applications - It will listen to the PORT number indicated by the PORT env variable and will make the original service listen in the localhost on PORT + 1. 
- Logs are printed to stdout.


### Traffic flow

The diagram below shows where gasket sits and how it manages requests. No ```iptables``` magic needed.

![diagram](gasket.png)

### Why ?

I wanted to have tls and mtls termination at the container and experiment with Rust to do it. I didn't wanted to adopt a Service Mesh for that and I believe that the mix of cgroups, namespaces and linux allow for a powerful ENTRYPOINT management tool to help on that.

### Build
$ cargo build --release

### Command line options
    -e (--execute) the command to be executed 
    -c (--cert) tls certificate path
    -t (--tls) Start server in TLS mode (https)
    -m (--mtls) Start server in mTLS mode (peer/client verification)

If -t or -m is not set gasket defaults to plain http. If -t and -m is set it defaults to mTLS.


### Inspiration

Actix, Tokio and OpenSSL documentation, [Mozilla TLS docs](https://wiki.mozilla.org/Security/Server_Side_TLS#Intermediate_compatibility_.28recommended.29), [Linkerd state of the art proxy](https://linkerd.io/2020/07/23/under-the-hood-of-linkerds-state-of-the-art-rust-proxy-linkerd2-proxy/), [mTLS example server](https://github.com/sjolicoeur/rust-mtls-example-server)
