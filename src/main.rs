use lazy_static::lazy_static;
use std::time::Duration;

use actix_web::{HttpRequest, HttpResponse, HttpMessage, client};
use actix_web::http::header::{HeaderName, HeaderMap};
use futures::{Stream, Future};


lazy_static! {
    static ref HEADER_X_FORWARDED_FOR: HeaderName = HeaderName::from_lowercase(b"x-forwarded-for").unwrap();

    static ref HOP_BY_HOP_HEADERS: Vec<HeaderName> = vec![
        HeaderName::from_lowercase(b"connection").unwrap(),
        HeaderName::from_lowercase(b"proxy-connection").unwrap(),
        HeaderName::from_lowercase(b"keep-alive").unwrap(),
        HeaderName::from_lowercase(b"proxy-authenticate").unwrap(),
        HeaderName::from_lowercase(b"proxy-authorization").unwrap(),
        HeaderName::from_lowercase(b"te").unwrap(),
        HeaderName::from_lowercase(b"trailer").unwrap(),
        HeaderName::from_lowercase(b"transfer-encoding").unwrap(),
        HeaderName::from_lowercase(b"upgrade").unwrap(),
    ];

    static ref HEADER_TE: HeaderName = HeaderName::from_lowercase(b"te").unwrap();

    static ref HEADER_CONNECTION: HeaderName = HeaderName::from_lowercase(b"connection").unwrap();
}

static DEFAULT_TIMEOUT: Duration = Duration::from_secs(60);

use crate::config::Config;
use actix_web::client::Client;
use actix_web::error as actix_error;
use actix_web::{middleware, web, App, HttpRequest, HttpResponse, HttpServer};
use error::{Error, Result};
use futures::{Future, IntoFuture};
use http::header;
use std::net::SocketAddr;
use url::Url;


const OUT_IP: [u8; 4] = [127, 0, 0, 1];
const CONFIG_PATH: &str = "config.toml";

fn main() -> Result<()> {
    let listen_addr = "127.0.0.1:8443";
    
    HttpServer::new(move || {
        App::new()
            .data(Client::new())
            .wrap(middleware::Logger::default())
            .default_service(web::route().to(forward))
    })
    .bind(listen_addr)
    .map_err(|source| Error::BindPort {
        listen_addr,
        source,
    })?
    .system_exit()
    .run()
    .map_err(|source| Error::Run { source })
}

async fn forward(
    req: HttpRequest,
    payload: web::Payload,
    client: web::Data<Client>,
) -> String { 
    map_request(req.clone())
        .map_err(actix_error::ErrorNotFound)
        .into_future()
        .and_then(move |new_url| {
            client
                .request_from(new_url.as_str(), req.head())
                .if_some(req.head().peer_addr, |addr, fr| {
                    fr.header("x-forwarded-for", format!("{}", addr.ip()))
                })
                .send_stream(payload)
                .map(|res| {
                    let mut client_resp = HttpResponse::build(res.status());
                    for (header_name, header_value) in
                        res.headers().iter().filter(|(h, _)| *h != "connection")
                    {
                        client_resp.header(header_name.clone(), header_value.clone());
                    }
                    client_resp.streaming(res)
                })
                .map_err(actix_error::Error::from)
        })
}

fn map_request(req: HttpRequest) -> Result<Url> {
    let socket_addr = req
        .headers()
        .get(header::HOST)
        .ok_or(Error::HostEmpty)
        .and_then(|host| get_addr(host))?;

    let path = req.uri().path_and_query().map(|x| x.as_str()).unwrap_or("");

    let url = format!("http://{}{}", socket_addr, path);
    url.parse::<Url>()
        .map_err(|source| Error::InvalidUpstreamUrl { url, source })
}

fn get_upstream(host_value: &header::HeaderValue) -> Result<SocketAddr> {
    let host = host_value
        .to_str()
        .map_err(|source| Error::HostReadError { source })?
        .to_owned();

    let out_port = "upstream:3000";
   
    Ok((OUT_IP, out_port).into())
}