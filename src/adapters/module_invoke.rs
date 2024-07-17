use homedir::get_my_home;
use tokio::sync::mpsc::Sender;
use wasmtime::{
    component::{Component, Linker, ResourceTable},
    Config, Engine, Store,
};
use wasmtime_wasi::{pipe::MemoryOutputPipe, WasiCtxBuilder};
use wasmtime_wasi_http::WasiHttpCtx;

use super::{
    module_events::ModuleEvent, module_imports::ModuleImports, wasi_http_linker, wasi_view::Wasi, wit::{exports::raikiri_wit::bindings::wasi_http::{ModuleResponse, Request}, Bindings}
};

pub async fn invoke_wasm_module(
    username_module_name: String,
    params: Vec<u8>,
    mut call_stack: Vec<String>,
    event_sender: Sender<ModuleEvent>,
) -> Result<ModuleResponse, Box<dyn std::error::Error>> {
    if call_stack.len() > 10 {
        return Ok(ModuleResponse {
            status: 400,
            body: "CALL STACK LIMIT SIZE REACHED".as_bytes().to_vec(),
            headers: vec![],
        });
    }
    if call_stack.contains(&username_module_name) {
        return Ok(ModuleResponse {
            status: 400,
            body: "CYCLIC CALL FORBIDDEN".as_bytes().to_vec(),
            headers: vec![],
        });
    }
    call_stack.push(username_module_name.clone());

    let homedir = get_my_home()?.unwrap();
    let homedir = homedir.to_str().unwrap();
    let wasm_path = format!("{homedir}/.raikiri/modules/{username_module_name}.aot.wasm");
    let mut config = Config::new();
    config.cache_config_load_default()?;
    config.wasm_backtrace_details(wasmtime::WasmBacktraceDetails::Enable);
    config.wasm_component_model(true);
    config.debug_info(true);
    let engine = Engine::new(&config)?;
    let mut linker = Linker::<Wasi<ModuleImports>>::new(&engine);
    linker.allow_shadowing(true);
    wasmtime_wasi::add_to_linker_sync(&mut linker)?;
    wasi_http_linker::add_to_linker_sync(&mut linker)?;
    Bindings::add_to_linker(&mut linker, |x| &mut x.data)?;
    let stdout = MemoryOutputPipe::new(0x4000);
    let module_imports = ModuleImports {
        call_stack,
        event_sender: event_sender.clone(),
    };
    let wasi_ctx = WasiCtxBuilder::new()
        .inherit_stdin()
        .stdout(stdout.clone())
        .inherit_args()
        .build();
    let wasi_http_ctx = WasiHttpCtx::new();
    let wasi: Wasi<ModuleImports> = Wasi {
        data: module_imports,
        table: ResourceTable::new(),
        ctx: wasi_ctx,
        http_ctx: wasi_http_ctx
    };
    let mut store: Store<Wasi<ModuleImports>> = Store::new(&engine, wasi);
    let component = unsafe { Component::deserialize_file(&engine, wasm_path)? };
    let (bindings, _) = Bindings::instantiate(&mut store, &component, &linker)?;
    let timer = tokio::time::timeout(
        // TODO: make timeout configurable
        tokio::time::Duration::from_millis(300),
        tokio::task::spawn_blocking(move || {
            bindings.raikiri_wit_bindings_wasi_http().call_handle_http(
                &mut store,
                &Request {
                    body: params,
                    headers: Vec::new(),
                },
            )
        }),
    )
    .await;
    event_sender
        .send(ModuleEvent::Stdout {
            stdout,
            username_module_name,
        })
        .await?;
    let runtime_result = match timer {
        Err(_) => {
            return Ok(ModuleResponse {
                status: 500,
                body: "EXECUTION TIMEOUT".as_bytes().to_vec(),
                headers: vec![],
            });
        }
        Ok(v) => v.unwrap(),
    };
    match runtime_result {
        Err(e) => {
            eprintln!("{e}");
            Ok(ModuleResponse {
                status: 500,
                body: format!("RUNTIME ERROR: {}", e).as_bytes().to_vec(),
                headers: vec![],
            })
        },
        Ok(v) => Ok(v),
    }
}
