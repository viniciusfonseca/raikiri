use futures::stream;
use http_body_util::{combinators::BoxBody, BodyExt, StreamBody};
use hyper::body::{Bytes, Frame};
use tokio::sync::mpsc::Sender;
use wasmtime_wasi_http::types::HostFutureIncomingResponse;

use super::{component_events::ComponentEvent, component_registry::ComponentRegistry, context::RaikiriContext, secret_storage, wasi_view::Wasi};

pub struct ComponentImports {
    pub call_stack: Vec<String>,
    pub event_sender: Sender<ComponentEvent>,
    pub component_registry: ComponentRegistry
}

impl Clone for ComponentImports {
    fn clone(&self) -> Self {
        Self {
            call_stack: self.call_stack.clone(),
            event_sender: self.event_sender.clone(),
            component_registry: self.component_registry.clone()
        }
    }
}

impl RaikiriContext for ComponentImports {
    fn call_stack(&self) -> &Vec<String> {
        &self.call_stack
    }

    fn event_sender(&self) -> &Sender<ComponentEvent> {
        &self.event_sender
    }

    fn component_registry(&self) -> &ComponentRegistry {
        &self.component_registry
    }
    
    fn handle_http(&self, request: hyper::Request<wasmtime_wasi_http::body::HyperOutgoingBody>,
        config: wasmtime_wasi_http::types::OutgoingRequestConfig,
    ) -> wasmtime_wasi_http::HttpResult<wasmtime_wasi_http::types::HostFutureIncomingResponse> {
        if request.uri().host().unwrap().eq("raikiri.components") {
            let data = self.clone();
            let username_component_name = request.uri().path().replace("/", "");
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
                let secrets = tokio::runtime::Handle::current().block_on( secret_storage::get_component_secrets(username_component_name.clone())).unwrap();
                Ok(super::component_invoke::invoke_component(username_component_name, request, Wasi::new(data, secrets)).await)
            });
            return Ok(HostFutureIncomingResponse::Pending(future_handle))
        }
        Ok(wasmtime_wasi_http::types::default_send_request(request, config))
    }
}