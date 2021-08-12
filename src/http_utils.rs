use actix_web::{HttpRequest, HttpResponse};
use log::info;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

const HEADER_X_FORWARDED_FOR: &str = "X-FORWARDED-FOR";
const HEADER_X_GASKET_REQUEST_ID: &str = "X-GASKET-REQUEST-ID";

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
        go: Arc<Mutex<crate::GasketOptions>>,
    ) -> actix_web::Result<actix_web::HttpResponse> {
        // create an exponential backoff for the URL Path of ot does not exists
        let backoff_key = req.uri().path().to_string().clone();

        // check if the circuitbreaker is open
        if !sp.lock().unwrap().check_cb_status(backoff_key.clone()) {
            info!("Circuitbreaker for {} tripped", backoff_key.clone());
            return Ok(HttpResponse::InternalServerError().body(format!(
                "Circuitbreaker for {} is open",
                backoff_key.clone()
            )));
        }

        sp.lock().unwrap().exponential_backoff(backoff_key.clone());

        let to = sp
            .lock()
            .unwrap()
            .current_timeout(backoff_key.clone())
            .to_std()
            .unwrap();

        let client = awc::Client::new();
        let mut new_url = url.clone();

        new_url.set_path(req.uri().path());
        new_url.set_query(req.uri().query());
        let mut client_req = client
            .request_from(new_url.as_str(), req.head())
            .no_decompress();

        // timeout increases on failures to avoid slowdowns
        client_req = client_req.timeout(to);

        client_req = if let Some(addr) = req.peer_addr() {
            client_req.append_header((HEADER_X_FORWARDED_FOR, format!("{}", addr.ip())))
        } else {
            client_req
        };
        // stamp unique id
        let id = Uuid::new_v4();
        client_req = client_req.append_header((HEADER_X_GASKET_REQUEST_ID, id.to_string()));

        let mut res = match client_req.send_body(body).await {
            Ok(res) => res,
            Err(awc::error::SendRequestError::Timeout) => {
                // Increments timeout if a timeout is received from backend
                let tt = sp
                    .lock()
                    .unwrap()
                    .next_backoff(backoff_key.clone())
                    .to_std()
                    .unwrap();
                return Ok(
                    HttpResponse::RequestTimeout().body(format!("Backend Timeout: {:?}", tt))
                );
            }
            Err(e) => {
                if sp.lock().unwrap().trip(backoff_key.clone()) {
                    info!("Circuitbreaker for {} tripped", backoff_key.clone());
                }
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
