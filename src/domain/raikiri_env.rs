use std::path;

use vfs::{async_vfs::{AsyncFileSystem, AsyncPhysicalFS}, error::VfsErrorKind, VfsMetadata, VfsResult};
use wasmtime::{component::Component, Config, Engine};
use yaml_rust2::{Yaml, YamlLoader};

use crate::adapters::raikirifs::ThreadSafeError;

pub struct RaikiriEnvironment {
    pub fs: Box<dyn AsyncFileSystem>,
    pub username: String,
    pub wasm_engine: Engine
}

impl RaikiriEnvironment {
    pub fn new() -> Self {
        let mut config = Config::new();
        config.cache_config_load_default().expect("could not load default cache config");
        config.wasm_backtrace_details(wasmtime::WasmBacktraceDetails::Enable);
        config.wasm_component_model(true);
        config.async_support(true);
        let wasm_engine = Engine::new(&config).expect("could not create engine");

        let fs = Box::new(AsyncPhysicalFS::new("/"));
        let username = whoami::username();
        Self { fs, username, wasm_engine }
    }
}