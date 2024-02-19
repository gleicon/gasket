use actix_web::{http::HeaderName, HttpRequest, HttpResponse};
use std::sync::{Arc, Mutex};

use uuid::Uuid;

const HEADER_X_FORWARDED_FOR: &str = "X-FORWARDED-FOR";
const HEADER_X_GASKET_REQUEST_ID: &str = "X-GASKET-REQUEST-ID";
const HEADER_X_GASKET_MTLS_ACTIVE: &str = "X-GASKET-MTLS-ACTIVE";

const HOP_BY_HOP_HEADERS: [&str; 9] = [
    "connection",
    "proxy-connection",
    "keep-alive",
    "proxy-authenticate",
    "proxy-authorization",
    "te",
    "trailer",
    "transfer-encoding",
    "upgrade",
];

pub struct Proxy {
    pub id: Uuid, // request unique id
    pub retries: u32,
}

impl Proxy {
    pub async fn forward(
        req: HttpRequest,
        body: actix_web::web::Bytes,
        url: url::Url,
        go: Arc<Mutex<crate::GasketOptions>>,
    ) -> actix_web::Result<actix_web::HttpResponse> {
        let client = awc::Client::new();
        let mut new_url = url.clone();

        new_url.set_path(req.uri().path());
        new_url.set_query(req.uri().query());
        let mut client_req = client
            .request_from(new_url.as_str(), req.head())
            .no_decompress();

        // prune hop-by-hop headers
        for header in HOP_BY_HOP_HEADERS {
            let hh = client_req.headers_mut();
            hh.remove(HeaderName::from_static(header));
        }

        client_req = if let Some(addr) = req.peer_addr() {
            client_req.append_header((HEADER_X_FORWARDED_FOR, format!("{}", addr.ip())))
        } else {
            client_req
        };
        // stamp unique id
        let id = Uuid::new_v4();
        client_req = client_req.append_header((HEADER_X_GASKET_REQUEST_ID, id.to_string()));
        client_req = client_req.append_header((
            HEADER_X_GASKET_MTLS_ACTIVE,
            go.lock().unwrap().mtls_enabled.to_string(),
        ));

        let mut res = match client_req.send_body(body).await {
            Ok(res) => res,
            Err(awc::error::SendRequestError::Timeout) => {
                // Increments timeout if a timeout is received from backend

                return Ok(HttpResponse::RequestTimeout().body("Backend Timeout"));
            }
            Err(e) => {
                return Ok(HttpResponse::InternalServerError().body(format!("{e}")));
            }
        };

        //let res_body = actix_web::dev::AnyBody::from(res.body().await.unwrap());
        let mut hrb = HttpResponse::build(res.status());

        for (header_name, header_value) in res.headers().iter().filter(|(h, _)| *h != "connection")
        {
            hrb.append_header((header_name.clone(), header_value.clone()));
        }
        hrb.append_header((HEADER_X_GASKET_REQUEST_ID, id.to_string()));
        let res_a = hrb.message_body(res.body().await.unwrap().into()).unwrap();

        Ok(res_a)
    }
}
