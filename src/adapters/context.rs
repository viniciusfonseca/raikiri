use tokio::sync::mpsc::Sender;
use wasmtime_wasi_http::{body::HyperOutgoingBody, types::{HostFutureIncomingResponse, OutgoingRequestConfig}, HttpResult};

use crate::ComponentEvent;

use super::component_registry::ComponentRegistry;

pub trait RaikiriContext {
    fn call_stack(&self) -> &Vec<String>;
    fn event_sender(&self) -> &Sender<ComponentEvent>;
    fn component_registry(&self) -> &ComponentRegistry;
    fn handle_http(&self, request: hyper::Request<HyperOutgoingBody>,
        config: OutgoingRequestConfig,
    ) -> HttpResult<HostFutureIncomingResponse>; 
}