use std::sync::Arc;

use tokio::net::TcpListener;
use wasmtime::{Config, Engine};

use crate::{adapters::{self, cache::Cache, component_registry, raikirifs::ThreadSafeError}, ComponentRegistry};

#[derive(Clone)]
pub struct RaikiriEnvironment {
    pub fs_root: String,
    pub username: String,
    pub wasm_engine: Engine,
    pub component_registry: ComponentRegistry,
    pub secrets_cache: Cache<String, Vec<(String, String)>>,
    pub port: u16,
    pub conf_file: adapters::conf_file::ConfFile
}

impl RaikiriEnvironment {
    pub fn new() -> Self {
        let mut config = Config::new();
        config.cache_config_load_default().expect("could not load default cache config");
        config.wasm_backtrace_details(wasmtime::WasmBacktraceDetails::Enable);
        config.wasm_component_model(true);
        config.async_support(true);
        let wasm_engine = Engine::new(&config).expect("could not create engine");

        let fs_root = format!("/home/{}/.raikiri", whoami::username());
        let username = whoami::username();
        Self {
            fs_root,
            username,
            wasm_engine,
            component_registry: adapters::cache::new_empty_cache(),
            secrets_cache: adapters::cache::new_empty_cache(),
            port: 0,
            conf_file: adapters::conf_file::ConfFile::build().unwrap()
        }
    }

    pub async fn init<T>(&mut self) -> Result<&mut Self, ThreadSafeError> {

        println!("Registering components...");
        self.component_registry = component_registry::build_registry().await?;
        println!("Successfully registered components");

        Ok(self)
    }

    pub fn with_username(&mut self, username: String) -> Self {
        self.username = username;
        self.clone()
    }

    pub fn with_fs_root(&mut self, fs_root: String) -> Self {
        self.fs_root = fs_root;
        self.clone()
    }

    pub fn with_port(&mut self, port: u16) -> Self {
        self.port = port;
        self.clone()
    }

}