use wasmtime_wasi::ResourceTable;
use wasmtime_wasi_http::{
    body::HyperOutgoingBody,
    types::{self, HostFutureIncomingResponse, OutgoingRequestConfig},
    HttpResult, WasiHttpCtx, WasiHttpView,
};

use super::{module_imports::ModuleImports, wasi_view::Wasi};

impl WasiHttpView for Wasi<ModuleImports> {
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
        Ok(types::default_send_request(request, config))
    }
}
