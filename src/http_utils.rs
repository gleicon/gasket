use actix_web::{HttpRequest, HttpResponse};
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
    ) -> actix_web::Result<actix_web::HttpResponse> {
        let client = awc::Client::new();
        let mut new_url = url.clone();

        new_url.set_path(req.uri().path());
        new_url.set_query(req.uri().query());
        let mut client_req = client
            .request_from(new_url.as_str(), req.head())
            .no_decompress();

        println!("oi");
        client_req = if let Some(addr) = req.peer_addr() {
            client_req.append_header((HEADER_X_FORWARDED_FOR, format!("{}", addr.ip())))
        } else {
            client_req
        };
        println!("oi");
        let id = Uuid::new_v4();
        client_req = client_req.append_header((HEADER_X_GASKET_REQUEST_ID, id.to_string()));
        let mut res = client_req.send_body(body).await.unwrap();
        println!("oi");
        let res_body = actix_web::dev::AnyBody::from(res.body().await.unwrap());

        let mut hrb = HttpResponse::build(res.status());

        println!("oi");
        // prune headers
        for (header_name, header_value) in res.headers().iter().filter(|(h, _)| *h != "connection")
        {
            hrb.append_header((header_name.clone(), header_value.clone()));
        }
        hrb.append_header((HEADER_X_GASKET_REQUEST_ID, id.to_string()));
        println!("oi");
        let res_a = hrb.message_body(res_body).unwrap();

        return Ok(res_a);
    }
}
