use wasmtime::{component::Component, Config, Engine};

use crate::adapters::raikirifs::get_raikiri_home;

use super::raikirifs;

pub async fn add_component(
    username: String,
    component_name: String,
    file_path: String,
) -> Result<String, Box<dyn std::error::Error>> {
    let file_content = tokio::fs::read(file_path).await?;
    add_component_bytes(username, component_name, &file_content).await
}

pub async fn add_component_bytes(
    username: String,
    component_name: String,
    component_bytes: &Vec<u8>,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut config: Config = Config::new();
    config.wasm_backtrace_details(wasmtime::WasmBacktraceDetails::Enable);
    config.wasm_component_model(true);
    config.async_support(true);
    let engine = Engine::new(&config)?;
    let component = Component::from_binary(&engine, component_bytes).expect("error compiling wasm component");
    let raikiri_home = get_raikiri_home()?;
    let aot_component_path = format!("{raikiri_home}/components/{username}.{component_name}.aot.wasm");
    let bytes = component
        .serialize()
        .expect("error serializing component to file");
    raikirifs::write_file(format!("components/{username}.{component_name}.aot.wasm"), bytes).await?;
    Ok(aot_component_path)
}

pub async fn remove_component(
    username: String,
    component_name: String
) -> Result<(), Box<dyn std::error::Error>> {
    raikirifs::remove_file(format!("components/{username}.{component_name}.aot.wasm")).await
}