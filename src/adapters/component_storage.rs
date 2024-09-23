use homedir::get_my_home;
use tokio::fs;
use wasmtime::{component::Component, Config, Engine};

pub async fn add_component(
    username: String,
    component_name: String,
    file_path: String,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut config = Config::new();
    config.cache_config_load_default()?;
    config.wasm_backtrace_details(wasmtime::WasmBacktraceDetails::Enable);
    config.wasm_component_model(true);
    config.debug_info(true);
    let engine = Engine::new(&config)?;
    let file_content = tokio::fs::read(file_path).await?;
    let component = Component::from_binary(&engine, &file_content).expect("error compiling wasm component");
    let homedir = get_my_home()?.unwrap();
    let homedir = homedir.to_str().unwrap();
    let aot_component_path = format!("{homedir}/.raikiri/components/{username}.{component_name}.aot.wasm");
    let bytes = component
        .serialize()
        .expect("error serializing component to file");
    fs::write(&aot_component_path, bytes).await?;
    Ok(aot_component_path)
}