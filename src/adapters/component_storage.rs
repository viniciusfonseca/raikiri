use homedir::get_my_home;
use tokio::fs;
use wasmtime::{component::Component, Config, Engine};

pub async fn add_component(
    username: String,
    component_name: String,
    file_path: String,
) -> Result<String, Box<dyn std::error::Error>> {
    let file_content = tokio::fs::read(file_path).await?;
    add_component_bytes(username, component_name, file_content).await
}

pub async fn add_component_bytes(
    username: String,
    component_name: String,
    component_bytes: Vec<u8>,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut config: Config = Config::new();
    config.wasm_backtrace_details(wasmtime::WasmBacktraceDetails::Enable);
    config.wasm_component_model(true);
    config.async_support(true);
    let engine = Engine::new(&config)?;
    let component = Component::from_binary(&engine, &component_bytes).expect("error compiling wasm component");
    let homedir = get_my_home()?.unwrap();
    let homedir = homedir.to_str().unwrap();
    let aot_component_path = format!("{homedir}/.raikiri/components/{username}.{component_name}.aot.wasm");
    let bytes = component
        .serialize()
        .expect("error serializing component to file");
    fs::write(&aot_component_path, bytes).await?;
    Ok(aot_component_path)
}

pub async fn remove_component(
    username: String,
    component_name: String
) -> Result<(), Box<dyn std::error::Error>> {

    let homedir = get_my_home()?.unwrap();
    let homedir = homedir.to_str().unwrap();
    let aot_component_path = format!("{homedir}/.raikiri/components/{username}.{component_name}.aot.wasm");
    fs::remove_file(aot_component_path).await?;

    Ok(())
}