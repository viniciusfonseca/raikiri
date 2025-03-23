use wasmtime::{component::Component, Config, Engine};

use crate::{adapters::cache::{new_empty_cache, Cache}, adapters::raikirifs::get_raikiri_home};

use super::raikirifs::ThreadSafeError;

pub type ComponentRegistry = Cache<String, Component>;

pub async fn build_registry() -> Result<ComponentRegistry, ThreadSafeError> {
    let component_registry = new_empty_cache();

    let raikiri_home = get_raikiri_home()?;
    let mut entries = tokio::fs::read_dir(format!("{raikiri_home}/components/")).await?;
    let mut config = Config::new();
    config.cache_config_load_default()?;
    config.wasm_backtrace_details(wasmtime::WasmBacktraceDetails::Enable);
    config.wasm_component_model(true);
    config.async_support(true);
    let engine = Engine::new(&config)?;

    while let Some(file) = entries.next_entry().await? {

        let filename = file.path().file_name().unwrap().to_str().unwrap().to_string().replace(".aot.wasm", "");
        component_registry.get_entry_by_key(filename.clone(), || {
            unsafe { Component::deserialize_file(&engine, file.path()).unwrap() }
        }).await;
        println!("successfully registered {filename}");
    }

    Ok(component_registry)
}
