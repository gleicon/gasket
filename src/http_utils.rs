use actix_web::http::header::HeaderMap;
use actix_web::{HttpRequest, HttpResponse};
use url::Url;
use uuid::Uuid;

const HEADER_X_FORWARDED_FOR: &str = "x-forwarded-for";
const HEADER_X_GASKET_REQUEST_ID: &str = "x-gasket-request-id";
const HEADER_X_GASKET_THROTTLE: &str = "x-gasket-throttle";

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

// prune hop by hop headers, create a new headermap with the rest
// add gasket id and throttle info
// consider adding mTLS info
pub struct ForwardRequestClientBuilder {
    pub id: Uuid, // request unique id
    new_header_map: HeaderMap,
    pub client_req: actix_web::client::ClientRequest,
}

impl ForwardRequestClientBuilder {
    pub fn new(
        req: actix_web::HttpRequest,
        client: &actix_web::client::Client,
        url: &url::Url,
    ) -> Self {
        let mut new_url = url.clone();
        new_url.set_path(req.uri().path());
        new_url.set_query(req.uri().query());
        let mut s = Self {
            id: Uuid::new_v4(),
            new_header_map: HeaderMap::new(),
            client_req: client
                .request_from(new_url.as_str(), req.head())
                .no_decompress(),
        };
        s.client_req = if let Some(addr) = req.head().peer_addr {
            s.client_req
                .header("x-forwarded-for", format!("{}", addr.ip()))
        } else {
            s.client_req
        };
        s.client_req = s
            .client_req
            .header(HEADER_X_GASKET_REQUEST_ID, s.id.to_string());
        s.filter_headers(req.headers());
        return s;
    }

    pub fn filter_headers(&mut self, headers: &HeaderMap) {
        self.new_header_map = headers.clone()
    }
}

pub struct HttpResponseClientBuilder {
    pub id: Uuid, // request unique id
    pub http_response_client: actix_web::dev::HttpResponseBuilder,
    res: actix_web::client::ClientResponse<
        actix_web::dev::Decompress<
            actix_web::dev::Payload<
                std::pin::Pin<
                    std::boxed::Box<
                        dyn futures::Stream<
                            Item = std::result::Result<
                                actix_web::web::Bytes,
                                actix_web::error::PayloadError,
                            >,
                        >,
                    >,
                >,
            >,
        >,
    >,
}

impl HttpResponseClientBuilder {
    pub fn new(
        res: actix_web::client::ClientResponse<
            actix_web::dev::Decompress<
                actix_web::dev::Payload<
                    std::pin::Pin<
                        std::boxed::Box<
                            dyn futures::Stream<
                                Item = std::result::Result<
                                    actix_web::web::Bytes,
                                    actix_web::error::PayloadError,
                                >,
                            >,
                        >,
                    >,
                >,
            >,
        >,
        id: Uuid,
    ) -> Self {
        let mut s = Self {
            id: id,
            http_response_client: HttpResponse::build(res.status()),
            res: res,
        };
        for (header_name, header_value) in
            s.res.headers().iter().filter(|(h, _)| *h != "connection")
        {
            s.http_response_client
                .header(header_name.clone(), header_value.clone());
        }
        s.http_response_client
            .header(HEADER_X_GASKET_REQUEST_ID, s.id.to_string());
        return s;
    }

    pub async fn client_response(&mut self) -> actix_web::HttpResponse {
        self.http_response_client
            .body(self.res.body().await.unwrap())
    }
}
