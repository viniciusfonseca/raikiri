use wasmtime_wasi::ResourceTable;
use wasmtime_wasi_http::{
    body::HyperOutgoingBody,
    types::{self, HostFutureIncomingResponse, OutgoingRequestConfig},
    HttpResult, WasiHttpCtx, WasiHttpView,
};

use super::{component_imports::ComponentImports, wasi_view::Wasi};

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
        println!("calling send_request from host: {request:?}");
        Ok(types::default_send_request(request, config))
    }
}
