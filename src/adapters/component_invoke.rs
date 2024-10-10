use std::time::{Duration, Instant};

use homedir::get_my_home;
    use http_body_util::{combinators::BoxBody, BodyExt};
use hyper::body::Bytes;
use wasmtime::{
    component::{Component, Linker},
    Config, Engine, Store,
};
use wasmtime_wasi_http::{bindings::http::types::Scheme, hyper_request_error, types::IncomingResponse, WasiHttpView};

use super::{
    component_events::ComponentEvent, context::RaikiriContext, wasi_http_view::stream_from_string, wasi_view::Wasi
};

async fn build_response(status: u16, body: &str) -> IncomingResponse {
    let resp = http::Response::builder()
        .status(status)
        .body(stream_from_string(body.to_string()).await)
        .map_err(|_| wasmtime_wasi_http::bindings::http::types::ErrorCode::ConnectionReadTimeout).unwrap()
        .map(|body| body.map_err(hyper_request_error).boxed());
    wasmtime_wasi_http::types::IncomingResponse {
        resp,
        worker: None,
        between_bytes_timeout: Duration::new(0, 0)
    }
}

pub async fn invoke_component<T>(
    username_component_name: String,
    req: hyper::Request<BoxBody<Bytes, hyper::Error>>,
    wasi: Wasi<T>,
) -> Result<IncomingResponse, wasmtime_wasi_http::bindings::http::types::ErrorCode>
    where T: Send + Clone + RaikiriContext + 'static,
{
    let start = Instant::now();
    let data = wasi.data.clone();
    let mut call_stack = data.call_stack().clone();

    if call_stack.len() > 10 {
        data.event_sender()
            .send(ComponentEvent::Execution {
                stdout: None,
                username_component_name,
                duration: start.elapsed().as_millis(),
                status: 400
            })
            .await.unwrap();
        return Ok(build_response(400, "CALL STACK LIMIT SIZE REACHED").await);
    }
    call_stack.push(username_component_name.clone());

    let homedir = get_my_home().unwrap().unwrap();
    let homedir = homedir.to_str().unwrap();
    let wasm_path = format!("{homedir}/.raikiri/components/{username_component_name}.aot.wasm");
    let call_stack_len = call_stack.len();
    let component_registry = wasi.data.component_registry();

    let component_entry = component_registry.get_entry_by_key(username_component_name.clone(), || {
        let mut config = Config::new();
        config.cache_config_load_default().unwrap();
        config.wasm_backtrace_details(wasmtime::WasmBacktraceDetails::Enable);
        config.wasm_component_model(true);
        config.async_support(true);
        let engine = Engine::new(&config).unwrap();
        unsafe { Component::deserialize_file(&engine, wasm_path.clone()).unwrap() }
    }).await;
    let component = component_entry.read().await;

    let stdout = wasi.stdout.clone();
    let mut store: Store<Wasi<T>> = Store::new(&component.engine(), wasi);
    let mut linker = Linker::<Wasi<T>>::new(&component.engine());
    linker.allow_shadowing(true);
    wasmtime_wasi::add_to_linker_async(&mut linker).unwrap();
    wasmtime_wasi_http::add_only_http_to_linker_async(&mut linker).unwrap();
    let proxy = wasmtime_wasi_http::bindings::Proxy::instantiate_async(&mut store, &component, &linker).await.unwrap();
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let out = store.data_mut().new_response_outparam(sender).unwrap();
    let req = store.data_mut().new_incoming_request(Scheme::Http, req).unwrap();
    let task = wasmtime_wasi::runtime::spawn(async move {
        proxy.wasi_http_incoming_handler().call_handle(&mut store, req, out).await
    });
    if call_stack_len == 1 {
        let timer = tokio::time::timeout(
            // TODO: make timeout configurable
            tokio::time::Duration::from_millis(300),
            task
        )
        .await;
        match timer {
            Err(_) => {
                return Ok(build_response(500, "EXECUTION TIMEOUT").await);
            }
            Ok(_) => (),
        };
    }
    else {
        task.await.unwrap();
    }

    let resp = match receiver.await {
        Ok(Ok(resp)) => {
            let (parts, body) = resp.into_parts();
            let collected = BodyExt::collect(body).await?;
            Some(Ok(hyper::Response::from_parts(parts, collected)))
        }
        Ok(Err(e)) => Some(Err(e)),
        Err(_) => None,
    }.expect("wasm never called set-response-outparam");
    let status;
    let v = match resp {
        Err(e) => {
            eprintln!("{e}");
            status = 500;
            Ok(build_response(500, &format!("RUNTIME ERROR: {}", e)).await)
        },
        Ok(v) => {
            status = v.status().as_u16();
            Ok(build_response(v.status().as_u16(), std::str::from_utf8(&v.into_body().to_bytes().to_vec()).unwrap()).await)
        }
    };
    data.event_sender()
        .send(ComponentEvent::Execution {
            stdout: Some(stdout),
            username_component_name,
            duration: start.elapsed().as_millis(),
            status
        })
        .await.unwrap();
    v
}
