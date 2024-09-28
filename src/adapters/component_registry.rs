use std::{collections::HashMap, sync::Arc};

use homedir::get_my_home;
use tokio::sync::RwLock;
use wasmtime::{component::Component, Config, Engine};

pub type ComponentRegistry = Arc<RwLock<HashMap<String, Component>>>;

pub async fn build_registry() -> Result<ComponentRegistry, Box<dyn std::error::Error>> {
    let mut component_registry = HashMap::<String, Component>::new();

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
        let component = unsafe { Component::deserialize_file(&engine, file.path()).unwrap() };
        let filename = file.path().file_name().unwrap().to_str().unwrap().to_string().replace(".aot.wasm", "");
        component_registry.insert(filename.clone(), component);
        println!("successfully registered {filename}");
    }

    Ok(Arc::new(RwLock::new(component_registry)))
}