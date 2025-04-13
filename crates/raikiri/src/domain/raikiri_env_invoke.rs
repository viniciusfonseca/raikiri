use std::time::Duration;

use async_trait::async_trait;
use http::Request;
use http_body_util::BodyExt;
use hyper::body::{Body, Bytes};
use wasmtime::{component::Linker, Store};
use wasmtime_wasi_http::{bindings::http::types::Scheme, hyper_request_error, types::IncomingResponse, WasiHttpView};

use crate::{adapters::{wasi_http_view::stream_from_string, context::RaikiriContext}, Wasi};

use super::{raikiri_env::{ComponentEvent, RaikiriEnvironment}, raikiri_env_component::RaikiriComponentStorage, raikiri_env_fs::RaikiriEnvironmentFS};

#[async_trait]
pub trait RaikiriEnvironmentInvoke {
    async fn invoke_component<T, B>(
        &self,
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
        &self,
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
            data.environment().event_sender.send(ComponentEvent::Execution {
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

        let call_stack_len = call_stack.len();
        let component_registry = &wasi.data.environment().component_registry;

        let (user, name) = username_component_name.split_once('.').unwrap();
        if !self.component_exists(user.to_string(), name.to_string()).await {
            return Ok(build_response(
                404,
                format!("Component {username_component_name} not found").as_str(),
            ).await)
        }

        let component_entry = component_registry
            .get_entry_by_key_async_build(username_component_name.clone(), async move {
                self.get_component(user.to_string(), name.to_string()).await.unwrap()
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
        let result = match resp {
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
        data.environment().event_sender.send(ComponentEvent::Execution {
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
        result
    }
}

async fn build_response(status: u16, body: &str) -> IncomingResponse {
    let resp = http::Response::builder()
        .status(status)
        .body(stream_from_string(body.to_string()).await)
        .map_err(|_| wasmtime_wasi_http::bindings::http::types::ErrorCode::ConnectionReadTimeout)
        .unwrap()
        .map(|body| body.map_err(hyper_request_error).boxed());
    wasmtime_wasi_http::types::IncomingResponse {
        resp,
        worker: None,
        between_bytes_timeout: Duration::new(0, 0),
    }
}