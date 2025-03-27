use std::sync::Arc;

use tokio::net::TcpListener;
use vfs::async_vfs::{AsyncFileSystem, AsyncPhysicalFS};
use wasmtime::{Config, Engine};

use crate::{adapters::cache::Cache, ComponentRegistry};

#[derive(Clone)]
pub struct RaikiriEnvironment {
    pub fs: Arc<dyn AsyncFileSystem>,
    pub username: String,
    pub wasm_engine: Engine,
    pub component_registry: Option<ComponentRegistry>,
    pub secrets_cache: Option<Cache<String, Vec<(String, String)>>>,
    pub port: Option<u16>,
}

impl RaikiriEnvironment {
    pub fn new() -> Self {
        let mut config = Config::new();
        config.cache_config_load_default().expect("could not load default cache config");
        config.wasm_backtrace_details(wasmtime::WasmBacktraceDetails::Enable);
        config.wasm_component_model(true);
        config.async_support(true);
        let wasm_engine = Engine::new(&config).expect("could not create engine");

        let fs = Arc::new(AsyncPhysicalFS::new("/"));
        let username = whoami::username();
        Self {
            fs,
            username,
            wasm_engine,
            component_registry: None,
            secrets_cache: None,
            port: None
        }
    }

    pub fn with_username(&mut self, username: String) -> Self {
        self.username = username;
        self.clone()
    }

    pub fn with_fs(&mut self, fs: Arc<dyn AsyncFileSystem>) -> Self {
        self.fs = fs;
        self.clone()
    }

}