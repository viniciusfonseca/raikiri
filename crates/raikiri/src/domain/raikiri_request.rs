use http::{HeaderMap, HeaderValue};

pub struct RaikiriServerRequest<T> {
    pub parts: http::request::Parts,
    pub body: T,
    pub method: String,
    pub headers: HeaderMap<HeaderValue>,
}
impl<T> RaikiriServerRequest<T> {
    pub fn from_parts(parts: http::request::Parts, body: T) -> Self {
        RaikiriServerRequest {
            parts: parts.clone(),
            body,
            method: parts.method.to_string(),
            headers: parts.headers,
        }
    }
}
