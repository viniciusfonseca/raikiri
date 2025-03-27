use async_trait::async_trait;
use http::Request;
use http_body_util::BodyExt;
use hyper::body::{Body, Bytes};
use wasmtime::{component::{Component, Linker}, Config, Engine, Store};
use wasmtime_wasi_http::{bindings::http::types::Scheme, types::IncomingResponse, WasiHttpView};

use crate::{build_response, get_raikiri_home, ComponentEvent, RaikiriContext, Wasi};

use super::raikiri_env::RaikiriEnvironment;

#[async_trait]
pub trait RaikiriEnvironmentInvoke {
    async fn invoke_component<T, B>(
        username_component_name: String,
        req: Request<B>,
        wasi: Wasi<T>,
    ) -> Result<IncomingResponse, wasmtime_wasi_http::bindings::http::types::ErrorCode>
    where
        T: Send + Clone + RaikiriContext + 'static,
        B: Body<Data = Bytes, Error = hyper::Error> + Send + Sync + 'static;
}

#[async_trait]
impl RaikiriEnvironmentInvoke for RaikiriEnvironment {
    
    async fn invoke_component<T, B>(
        username_component_name: String,
        req: Request<B>,
        wasi: Wasi<T>,
    ) -> Result<IncomingResponse, wasmtime_wasi_http::bindings::http::types::ErrorCode>
    where
        T: Send + Clone + RaikiriContext + 'static,
        B: Body<Data = Bytes, Error = hyper::Error> + Send + Sync + 'static,
    {
        let start = chrono::Utc::now();
        let data = wasi.data.clone();
        let mut call_stack = data.call_stack().clone();

        if call_stack.len() > 10 {
            data.event_sender()
                .send(ComponentEvent::Execution {
                    stdout: None,
                    username_component_name,
                    start,
                    duration: chrono::Utc::now()
                        .signed_duration_since(start)
                        .num_milliseconds(),
                    status: 400,
                })
                .await
                .unwrap();
            return Ok(build_response(400, "CALL STACK LIMIT SIZE REACHED").await);
        }
        call_stack.push(username_component_name.clone());

        let raikiri_home = get_raikiri_home().unwrap();
        let wasm_path = format!("{raikiri_home}/components/{username_component_name}.aot.wasm");
        let call_stack_len = call_stack.len();
        let component_registry = wasi.data.component_registry();

        let component_entry = component_registry
            .get_entry_by_key(username_component_name.clone(), || {
                let mut config = Config::new();
                config.cache_config_load_default().unwrap();
                config.wasm_backtrace_details(wasmtime::WasmBacktraceDetails::Enable);
                config.wasm_component_model(true);
                config.async_support(true);
                let engine = Engine::new(&config).unwrap();
                unsafe { Component::deserialize_file(&engine, wasm_path.clone()).unwrap() }
            })
            .await;
        let component = component_entry.read().await;

        let stdout = wasi.stdout.clone();
        let mut store = Store::new(&component.engine(), wasi);
        let mut linker = Linker::<Wasi<T>>::new(&component.engine());
        linker.allow_shadowing(true);
        wasmtime_wasi::add_to_linker_async(&mut linker).unwrap();
        wasmtime_wasi_http::add_only_http_to_linker_async(&mut linker).unwrap();
        let proxy =
            wasmtime_wasi_http::bindings::Proxy::instantiate_async(&mut store, &component, &linker)
                .await
                .unwrap();
        let (sender, receiver) = tokio::sync::oneshot::channel();
        let out = store.data_mut().new_response_outparam(sender).unwrap();
        let req = store
            .data_mut()
            .new_incoming_request(Scheme::Http, req)
            .unwrap();

        let task = wasmtime_wasi::runtime::spawn(async move {
            proxy
                .wasi_http_incoming_handler()
                .call_handle(&mut store, req, out)
                .await
        });
        if call_stack_len == 1 {
            let timeout = std::env::var("RAIKIRI_TIMEOUT")
                .unwrap_or_else(|_| "300".to_string())
                .parse::<u64>()
                .unwrap_or_else(|_| 300);
            let timer = tokio::time::timeout(
                // TODO: make timeout configurable
                tokio::time::Duration::from_millis(timeout),
                task,
            )
            .await;
            match timer {
                Err(_) => {
                    return Ok(build_response(500, "EXECUTION TIMEOUT").await);
                }
                Ok(_) => (),
            };
        } else {
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
        }
        .expect("wasm never called set-response-outparam");
        let status;
        let v = match resp {
            Err(e) => {
                eprintln!("{e}");
                status = 500;
                Ok(build_response(500, &format!("RUNTIME ERROR: {}", e)).await)
            }
            Ok(v) => {
                status = v.status().as_u16();
                Ok(build_response(
                    v.status().as_u16(),
                    std::str::from_utf8(&v.into_body().to_bytes().to_vec()).unwrap(),
                )
                .await)
            }
        };
        data.event_sender()
            .send(ComponentEvent::Execution {
                stdout: Some(stdout),
                username_component_name,
                start,
                duration: chrono::Utc::now()
                    .signed_duration_since(start)
                    .num_milliseconds(),
                status,
            })
            .await
            .unwrap();
        v
    }
}