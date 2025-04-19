use futures::stream;
use http_body_util::{combinators::BoxBody, StreamBody};
use hyper::body::{Bytes, Frame};
use wasmtime_wasi::{IoView, ResourceTable};
use wasmtime_wasi_http::{
    body::HyperOutgoingBody,
    types::{HostFutureIncomingResponse, OutgoingRequestConfig},
    HttpResult, WasiHttpCtx, WasiHttpView,
};

use super::{context::RaikiriContext, wasi_view::Wasi};

pub async fn stream_from_string(body: String) -> BoxBody<Bytes, hyper::Error> {
    BoxBody::new(StreamBody::new(stream::iter(
        body.into_bytes().to_vec().chunks(16 * 1024)
            .map(|chunk| Ok::<_, hyper::Error>(Frame::data(Bytes::copy_from_slice(chunk))))
            .collect::<Vec<_>>()
    )))
}

impl <T> IoView for Wasi<T> where T: Send + Clone + RaikiriContext + 'static {
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }
}

impl <T> WasiHttpView for Wasi<T> where T: Send + Clone + RaikiriContext + 'static {
    fn ctx(&mut self) -> &mut WasiHttpCtx {
        &mut self.http_ctx
    }

    fn send_request(
        &mut self,
        request: hyper::Request<HyperOutgoingBody>,
        config: OutgoingRequestConfig,
    ) -> HttpResult<HostFutureIncomingResponse> {
        self.data.handle_http(request, config)
    }
}
