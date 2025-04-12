use async_trait::async_trait;
use wasmtime::{component::Component, Config, Engine};

use crate::{adapters::{cache::Cache, raikirifs::ThreadSafeError}, new_empty_cache};

use super::{raikiri_env::RaikiriEnvironment, raikiri_env_fs::RaikiriEnvironmentFS};

pub type ComponentRegistry = Cache<String, Component>;

#[async_trait]
pub trait RaikiriComponentStorage {
    async fn add_component(&self, user: String, name: String, path: String) -> Result<(), ThreadSafeError>;
    async fn get_component(&self, user: String, name: String) -> Result<Component, ThreadSafeError>;
    async fn remove_component(&self, user: String, name: String) -> Result<(), ThreadSafeError>;
    async fn build_registry(&self) -> Result<ComponentRegistry, ThreadSafeError>;
}

#[async_trait]
impl RaikiriComponentStorage for RaikiriEnvironment {
    async fn add_component(&self, user: String, name: String, path: String) -> Result<(), ThreadSafeError> {
        let component_bytes = tokio::fs::read(path).await?;
        let component = Component::from_binary(&self.wasm_engine, &component_bytes).expect("error compiling wasm component");
        let component_bytes = component.serialize().expect("error serializing component to file");
        self.write_file(format!("components/{user}.{name}.aot.wasm"), component_bytes).await
    }

    async fn get_component(&self, user: String, name: String) -> Result<Component, ThreadSafeError> {
        let component_bytes = self.read_file(format!("components/{user}.{name}.aot.wasm")).await?;
        unsafe { Ok(Component::deserialize(&self.wasm_engine, &component_bytes).expect("error compiling wasm component")) }
    }

    async fn remove_component(&self, user: String, name: String) -> Result<(), ThreadSafeError> {
        self.remove_file(format!("components/{user}.{name}.aot.wasm")).await
    }

    async fn build_registry(&self) -> Result<ComponentRegistry, ThreadSafeError> {
        let component_registry = new_empty_cache();
    
        let entries = self.read_dir("components").await?;
        let mut config = Config::new();
        config.cache_config_load_default()?;
        config.wasm_backtrace_details(wasmtime::WasmBacktraceDetails::Enable);
        config.wasm_component_model(true);
        config.async_support(true);
        let engine = Engine::new(&config)?;
    
        for filename in entries {
            component_registry.get_entry_by_key(filename.clone(), || {
                unsafe { Component::deserialize_file(&engine, format!("{}/components/{}", self.fs_root, filename.clone())).unwrap() }
            }).await;
            println!("successfully registered {filename}");
        }
    
        Ok(component_registry)
    }
    
}