use actix_web::{web, HttpRequest, HttpResponse, Result};
use log::info;
use std::sync::{Arc, Mutex};
use url::Url;

pub async fn forward(
    req: HttpRequest,
    body: web::Bytes,
    dest_port: web::Data<Arc<u16>>,
    go: web::Data<Arc<Mutex<crate::GasketOptions>>>,
) -> Result<HttpResponse, actix_web::Error> {
    info!("request proxy");
    let dest_port = dest_port.as_ref();
    let go = go.as_ref();
    let forward_url = Url::parse(&format!("http://127.0.0.1:{dest_port}")).unwrap();
    crate::http_utils::Proxy::forward(req, body, &forward_url, go.clone()).await
}
