use actix_web::{web, HttpRequest, HttpResponse, Result};
use log::info;
use std::sync::{Arc, Mutex};
use tokio_retry::strategy::{jitter, ExponentialBackoff};
use tokio_retry::Retry;
use url::Url;

pub async fn forward(
    req: HttpRequest,
    body: web::Bytes,
    dest_port: web::Data<Arc<u16>>,
    go: web::Data<Arc<Mutex<crate::GasketOptions>>>,
) -> Result<HttpResponse, actix_web::Error> {
    let retry_strategy = ExponentialBackoff::from_millis(10)
        .map(jitter) // add jitter to delays
        .take(3); // limit to 3 retries
    info!("request proxy");
    let dest_port = dest_port.as_ref();
    let go = go.as_ref();

    Retry::spawn(retry_strategy, move || {
        let forward_url = Url::parse(&format!("http://127.0.0.1:{dest_port}")).unwrap();

        crate::http_utils::Proxy::forward(req.clone(), body.clone(), forward_url, go.clone())
    })
    .await

    //crate::http_utils::Proxy::forward(req, body, &forward_url, go.clone()).await
}
