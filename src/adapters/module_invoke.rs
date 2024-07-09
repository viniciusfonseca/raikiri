use exports::raikiri_wit::bindings::wasi_http::{Request, ModuleResponse};
use futures::executor::block_on;
use homedir::get_my_home;
use raikiri_wit::bindings::wasi_http::{Body, Headers};
use wasmtime::{component::{Component, Linker, ResourceTable}, Config, Engine, Store};
use wasmtime_wasi::{pipe::MemoryOutputPipe, WasiCtx, WasiCtxBuilder, WasiView};

wasmtime::component::bindgen!();

struct AppCtx {}
struct Wasi<T: Send>(T, AppCtx, ResourceTable, WasiCtx);

impl WasiView for Wasi<ModuleImports> {
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.2
    }

    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.3
    }
}

#[derive(Default)]
pub struct ModuleImports {
    call_stack: Vec<String>
}

impl raikiri_wit::bindings::wasi_http::Host for ModuleImports {

    fn handle_http(&mut self, _: raikiri_wit::bindings::wasi_http::Request) -> raikiri_wit::bindings::wasi_http::ModuleResponse {
        todo!()
    }

    fn call_module(
        &mut self,
        module_name: wasmtime::component::__internal::String,
        params: Body,
    ) -> raikiri_wit::bindings::wasi_http::ModuleResponse {
        let result = block_on(invoke_wasm_module(
            module_name,
            params.to_vec(),
            self.call_stack.clone(),
        ))
        .expect("error retrieving module result");
        raikiri_wit::bindings::wasi_http::ModuleResponse {
            status: result.status,
            body: result.body,
            headers: result
                .headers
                .iter()
                .map(|header| (header.0.clone(), header.1.clone()))
                .collect::<Headers>(),
        }
    }
}

pub async fn invoke_wasm_module(
    username_module_name: String,
    params: Vec<u8>,
    mut call_stack: Vec<String>,
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
    let engine = Engine::new(&config)?;
    let mut linker = Linker::<Wasi<ModuleImports>>::new(&engine);
    Bindings::add_to_linker(&mut linker, |x| &mut x.0)?;
    wasmtime_wasi::add_to_linker_sync(&mut linker)?;
    let stdout = MemoryOutputPipe::new(0x4000);
    let module_imports = ModuleImports { call_stack };
    let wasi_ctx = WasiCtxBuilder::new()
        .inherit_stdin()
        .stdout(stdout.clone())
        .inherit_args()
        .build();
    let wasi: Wasi<ModuleImports> = Wasi(module_imports, AppCtx {}, ResourceTable::new(), wasi_ctx);
    unsafe {
        let component = Component::deserialize_file(&engine, wasm_path)?;
        let mut store: Store<Wasi<ModuleImports>> = Store::new(&engine, wasi);
        let (bindings, _) = Bindings::instantiate(&mut store, &component, &linker)?;
        let timer = tokio::time::timeout(
            // TODO: make timeout configurable
            tokio::time::Duration::from_millis(300),
            tokio::task::spawn_blocking(move || {
                bindings.raikiri_wit_bindings_wasi_http().call_handle_http(&mut store, &Request {
                    body: params,
                    headers: Vec::new()
                })
            }),
        )
        .await;
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
        let module_result = match runtime_result {
            Err(e) => {
                eprintln!("{e}");
                return Ok(ModuleResponse {
                    status: 500,
                    body: format!("RUNTIME ERROR: {}", e)
                        .as_bytes()
                        .to_vec(),
                    headers: vec![],
                });
            }
            Ok(v) => v,
        };
        println!("result: {}", String::from_utf8(module_result.body.to_vec())?);
        println!("stoud: {}", String::from_utf8(stdout.contents().to_vec())?);
        Ok(module_result)
    }
}