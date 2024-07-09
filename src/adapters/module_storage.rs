use homedir::get_my_home;
use tokio::fs;
use wasmtime::{component::Component, Config, Engine};

pub async fn add_module(
    username: String,
    module_name: String,
    file_path: String,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut config = Config::new();
    config.cache_config_load_default()?;
    config.wasm_backtrace_details(wasmtime::WasmBacktraceDetails::Enable);
    config.wasm_component_model(true);
    let engine = Engine::new(&config)?;
    let file_content = tokio::fs::read(file_path).await?;
    let module = Component::from_binary(&engine, &file_content).expect("error compiling wasm module");
    let homedir = get_my_home()?.unwrap();
    let homedir = homedir.to_str().unwrap();
    let aot_module_path = format!("{homedir}/.raikiri/modules/{username}.{module_name}.aot.wasm");
    let bytes = module
        .serialize()
        .expect("error serializing module to file");
    fs::write(&aot_module_path, bytes).await?;
    Ok(aot_module_path)
}
