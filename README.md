### Gasket - container proxy and PID1 manager

#### What is it ?

Gasket is both a proxy which can terminate TLS and mTLS connections and a PID1 manager.

#### How ?

    - It is meant to be used at a containers ENTRYPOINT to wrap any 12 factor service or application.
    - Its architecture behaves as an PID1 manager to spawn a TLS aware proxy and then the service, with signal management and process supervision. 
    - It makes the original service listen in a local port, listen in the original env var PORT and proxies requests locally with low overhead.

˜[diagram](gasket.png)

#### Why ?

    - I wanted to have tls and mtls termination at the container and experiment with Rust to do it. I didn't wanted to adopt a Service MEsh for that and I believe that the mix of cgroups, namespaces and linux allow for a powerful ENTRYPOINT management tool to help on that.

#### Build
$ cargo build --release
