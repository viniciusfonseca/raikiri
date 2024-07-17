use futures::executor::block_on;
use tokio::sync::mpsc::Sender;

use super::{
    module_events::ModuleEvent,
    module_invoke::invoke_wasm_module,
    wit::raikiri_wit::{
        self,
        bindings::wasi_http::{Body, Headers},
    },
};

pub struct ModuleImports {
    pub call_stack: Vec<String>,
    pub event_sender: Sender<ModuleEvent>,
}

impl raikiri_wit::bindings::wasi_http::Host for ModuleImports {
    fn handle_http(
        &mut self,
        _: raikiri_wit::bindings::wasi_http::Request,
    ) -> raikiri_wit::bindings::wasi_http::ModuleResponse {
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
            self.event_sender.clone(),
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
