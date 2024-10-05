use wasmtime_wasi::{ResourceTable, WasiCtx, WasiView};
use wasmtime_wasi_http::WasiHttpCtx;

use super::component_imports::ComponentImports;

pub struct Wasi<T: Send + Clone> {
    pub data: T,
    pub table: ResourceTable,
    pub ctx: WasiCtx,
    pub http_ctx: WasiHttpCtx,
}

impl WasiView for Wasi<ComponentImports> {
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }

    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.ctx
    }
}
