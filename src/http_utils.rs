use actix_web::{HttpRequest, HttpResponse};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

const HEADER_X_FORWARDED_FOR: &str = "x-forwarded-for";
const HEADER_X_GASKET_REQUEST_ID: &str = "x-gasket-request-id";

// const HOP_BY_HOP_HEADERS: Vec<&str> = vec![
//     "connection",
//     "proxy-connection",
//     "keep-alive",
//     "proxy-authenticate",
//     "proxy-authorization",
//     "te",
//     "trailer",
//     "transfer-encoding",
//     "upgrade",
// ];

// TODO: prune hop by hop headers
// TODO: add throttle info
// TODO: consider adding mTLS info
pub struct Proxy {
    pub id: Uuid, // request unique id
}

impl Proxy {
    pub async fn forward(
        req: HttpRequest,
        body: actix_web::web::Bytes,
        url: &url::Url,
        sp: Arc<Mutex<crate::stability_patterns::StabilityPatterns>>,
    ) -> actix_web::Result<actix_web::HttpResponse> {
        // create an exponential backoff for the URL Path
        let to = sp
            .lock()
            .unwrap()
            .exponential_backoff(url.path().to_string());
        // connector w/ timeout
        let connector = awc::Connector::new()
            // This is the timeout setting for connector. It's 1 second by default
            .timeout(
                sp.lock()
                    .unwrap()
                    .next_backoff(url.path().to_string())
                    .to_std()
                    .unwrap(),
            )
            .finish();
        let client = awc::Client::new();
        let mut new_url = url.clone();

        new_url.set_path(req.uri().path());
        new_url.set_query(req.uri().query());
        let mut client_req = client
            .request_from(new_url.as_str(), req.head())
            .no_decompress();

        client_req = if let Some(addr) = req.peer_addr() {
            client_req.append_header((HEADER_X_FORWARDED_FOR, format!("{}", addr.ip())))
        } else {
            client_req
        };
        let id = Uuid::new_v4();
        client_req = client_req.append_header((HEADER_X_GASKET_REQUEST_ID, id.to_string()));
        let mut res = match client_req.send_body(body).await {
            Ok(res) => res,
            Err(e) => {
                return Ok(HttpResponse::InternalServerError().body(format!("{}", e)));
            }
        };

        let res_body = actix_web::dev::AnyBody::from(res.body().await.unwrap());

        let mut hrb = HttpResponse::build(res.status());

        for (header_name, header_value) in res.headers().iter().filter(|(h, _)| *h != "connection")
        {
            hrb.append_header((header_name.clone(), header_value.clone()));
        }
        hrb.append_header((HEADER_X_GASKET_REQUEST_ID, id.to_string()));
        let res_a = hrb.message_body(res_body).unwrap();

        return Ok(res_a);
    }
}
