use futures::stream;
use hashlink::LinkedHashMap;
use http::HeaderValue;
use http_body_util::{combinators::BoxBody, StreamBody};
use hyper::body::{Bytes, Frame};
use yaml_rust2::Yaml;

pub struct InvokeRequest {
    pub username_component_name: String,
    pub method: String,
    pub headers: LinkedHashMap<Yaml, Yaml>,
    pub body: Vec<u8>
}

impl InvokeRequest {
    pub fn new(username_component_name: String, method: String, headers: LinkedHashMap<Yaml, Yaml>, body: Vec<u8>) -> InvokeRequest {
        InvokeRequest {
            username_component_name,
            method,
            headers,
            body
        }
    }
}

impl Into<hyper::Request<BoxBody<hyper::body::Bytes, hyper::Error>>> for InvokeRequest where {
    fn into(self) -> hyper::Request<BoxBody<hyper::body::Bytes, hyper::Error>> {
        let username_component_name = self.username_component_name;
        let mut request_builder = hyper::Request::builder()
            .method(self.method.as_str())
            .uri(format!("http://raikiri.components/{}", username_component_name));
        for (k, v) in self.headers {
            request_builder = request_builder.header(k.as_str().unwrap(), HeaderValue::from_str(v.as_str().unwrap()).unwrap());
        }
        request_builder.body(BoxBody::new(StreamBody::new(stream::iter(
            self.body.chunks(16 * 1024)
                .map(|chunk| Ok::<_, hyper::Error>(Frame::data(Bytes::copy_from_slice(chunk))))
                .collect::<Vec<_>>()
        )))).unwrap()
    }
}