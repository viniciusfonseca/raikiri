use wasmtime_wasi::{pipe::MemoryOutputPipe, ResourceTable, WasiCtx, WasiCtxBuilder, WasiView};
use wasmtime_wasi_http::WasiHttpCtx;

use super::context::RaikiriContext;

pub struct Wasi<T: Send + Clone> {
    pub data: T,
    pub table: ResourceTable,
    pub ctx: WasiCtx,
    pub http_ctx: WasiHttpCtx,
    pub stdout: MemoryOutputPipe
}

impl <T> Wasi<T> where T: Send + Clone + RaikiriContext {
    pub fn new(data: T, envs: Vec<(String, String)>) -> Wasi<T> {
        let stdout = MemoryOutputPipe::new(0x4000);
        let ctx = WasiCtxBuilder::new()
            .inherit_stdin()
            .stdout(stdout.clone())
            .envs(&envs)
            .inherit_args()
            .build();
        let table = ResourceTable::new();
        let http_ctx = WasiHttpCtx::new();
        Self { data, table, ctx, http_ctx, stdout }
    }
}

impl <T> WasiView for Wasi<T> where T: Send + Clone + RaikiriContext + 'static {
    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.ctx
    }
}
