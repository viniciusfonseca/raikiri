use futures::stream;
use http_body_util::{combinators::BoxBody, BodyExt, StreamBody};
use hyper::body::{Bytes, Frame};
use wasmtime_wasi::ResourceTable;
use wasmtime_wasi_http::{
    body::HyperOutgoingBody,
    types::{self, HostFutureIncomingResponse, OutgoingRequestConfig},
    HttpResult, WasiHttpCtx, WasiHttpView,
};

use super::{component_imports::ComponentImports, wasi_view::Wasi, component_invoke::invoke_component};

pub async fn stream_from_string(body: String) -> BoxBody<Bytes, hyper::Error> {
    BoxBody::new(StreamBody::new(stream::iter(
        body.into_bytes().to_vec().chunks(16 * 1024)
            .map(|chunk| Ok::<_, hyper::Error>(Frame::data(Bytes::copy_from_slice(chunk))))
            .collect::<Vec<_>>()
    )))
}

impl WasiHttpView for Wasi<ComponentImports> {
    fn ctx(&mut self) -> &mut WasiHttpCtx {
        &mut self.http_ctx
    }

    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }

    fn send_request(
        &mut self,
        request: hyper::Request<HyperOutgoingBody>,
        config: OutgoingRequestConfig,
    ) -> HttpResult<HostFutureIncomingResponse> {
        if request.uri().host().unwrap().eq("raikiri.components") {
            let username_component_name = request.uri().path().replace("/", "");
            let call_stack = self.data.call_stack.clone();
            let event_sender = self.data.event_sender.clone();
            let future_handle = wasmtime_wasi::runtime::spawn(async move {
                let mut request_builder = hyper::Request::builder()
                    .uri(request.uri());
                for (key, value) in request.headers() {
                    request_builder = request_builder.header(key, value);
                }
                let body = request.into_body().collect().await.unwrap().to_bytes().to_vec();
                let request = request_builder.body(BoxBody::new(StreamBody::new(stream::iter(
                    body.chunks(16 * 1024)
                        .map(|chunk| Ok::<_, hyper::Error>(Frame::data(Bytes::copy_from_slice(chunk))))
                        .collect::<Vec<_>>()
                )))).unwrap();
                Ok(invoke_component(username_component_name, request, call_stack, event_sender).await)
            });
            return Ok(HostFutureIncomingResponse::Pending(future_handle))
        }
        Ok(types::default_send_request(request, config))
    }
}
