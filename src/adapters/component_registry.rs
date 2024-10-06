use homedir::get_my_home;
use wasmtime::{component::Component, Config, Engine};

use crate::adapters::cache::{Cache, new_empty_cache};

pub type ComponentRegistry = Cache<String, Component>;

pub async fn build_registry() -> Result<ComponentRegistry, Box<dyn std::error::Error>> {
    let component_registry = new_empty_cache();

    let homedir = get_my_home()?.unwrap();
    let homedir = homedir.to_str().unwrap();
    let mut entries = tokio::fs::read_dir(format!("{homedir}/.raikiri/components/")).await?;
    let mut config = Config::new();
    config.cache_config_load_default()?;
    config.wasm_backtrace_details(wasmtime::WasmBacktraceDetails::Enable);
    config.wasm_component_model(true);
    config.async_support(true);
    let engine = Engine::new(&config)?;

    while let Some(file) = entries.next_entry().await? {

        let filename = file.path().file_name().unwrap().to_str().unwrap().to_string().replace(".aot.wasm", "");
        component_registry.get_entry_by_key(filename.clone(), || {
            let component = unsafe { Component::deserialize_file(&engine, file.path()).unwrap() };
            component
        }).await;
        println!("successfully registered {filename}");
    }

    Ok(component_registry)
}
