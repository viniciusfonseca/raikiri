use std::net::SocketAddr;

use async_trait::async_trait;
use futures::stream;
use http::{Request, Response};
use http_body_util::{combinators::BoxBody, StreamBody};
use hyper::{body::{Body, Bytes, Frame, Incoming}, server::conn::http1, service::service_fn};
use tokio::net::TcpListener;
use wasmtime_wasi_http::{bindings::http::types::ErrorCode, io::TokioIo};


use crate::{adapters::{component_invoke, component_registry, raikirifs::ThreadSafeError, secret_storage}, default_event_handler, new_empty_cache, ComponentEvent, ComponentImports, Wasi};

use super::raikiri_env::RaikiriEnvironment;

#[async_trait]
pub trait RaikiriEnvironmentServer {
    async fn response_body<E, T>(body: T) -> BoxBody<Bytes, E>
    where
        E: Send + Sync + 'static,
        T: ToString + Send;
    async fn run_server(&self) -> Result<(), ThreadSafeError>;
}

#[async_trait]
impl RaikiriEnvironmentServer for RaikiriEnvironment {

    async fn response_body<E, T>(body: T) -> BoxBody<Bytes, E>
    where
        E: Send + Sync + 'static,
        T: ToString + Send,
    {
        BoxBody::new(StreamBody::new(stream::iter(
            body.to_string()
                .into_bytes()
                .to_vec()
                .chunks(16 * 1024)
                .map(|chunk| Ok::<_, E>(Frame::data(Bytes::copy_from_slice(chunk))))
                .collect::<Vec<_>>(),
        )))
    }
    async fn run_server(&self) -> Result<(), ThreadSafeError> {
        let addr = SocketAddr::from(([127, 0, 0, 1], self.port));
        let listener = TcpListener::bind(addr).await?;
        loop {
            let (stream, _) = listener.accept().await?;
            let io = TokioIo::new(stream);
            let self = self.clone();
    
            tokio::task::spawn(async move {
                if let Err(err) = http1::Builder::new()
                    .serve_connection(io, service_fn(|req| handle_request::<Incoming>(self.clone(), req)))
                    .await
                {
                    eprintln!("Error serving connection: {:?}", err);
                }
            });
        }
    }
}

async fn handle_request<B>(_self: RaikiriEnvironment, request: Request<B>) ->
    Result<Response<BoxBody<Bytes, ErrorCode>>, ThreadSafeError>
    where
        B: Body<Data = Bytes, Error = hyper::Error> + Send + Sync + 'static
{
    let command = request
            .headers()
            .get("Platform-Command")
            .unwrap()
            .to_str()
            .unwrap();

        match command {
            "Invoke-Component" => {
                let username_component_name = request
                    .headers()
                    .get("Component-Id")
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_string();

                let (tx, mut rx) = tokio::sync::mpsc::channel::<ComponentEvent>(0xFFFF);
                tokio::spawn(async move {
                    while let Some(message) = rx.recv().await {
                        default_event_handler(message)
                    }
                });
                let secrets_entry = &_self
                    .secrets_cache
                    .get_entry_by_key_async_build(username_component_name.clone(), async {
                        secret_storage::get_component_secrets(username_component_name.clone())
                            .await
                            .unwrap_or_else(|_| Vec::new())
                    })
                    .await;
                let secrets = secrets_entry.read().await;
                let component_imports = ComponentImports {
                    call_stack: Vec::new(),
                    component_registry: _self.component_registry.clone(),
                    event_sender: tx,
                    secrets_cache: _self.secrets_cache.clone(),
                };
                let response = component_invoke::invoke_component(
                    username_component_name.clone(),
                    request,
                    Wasi::new(component_imports, secrets.to_vec()),
                )
                .await
                .unwrap();

                let (parts, body) = response.resp.into_parts();
                Ok(hyper::Response::from_parts(parts, body))
            }
            _ => {
                return Ok(Response::builder()
                    .status(404)
                    .body(RaikiriEnvironment::response_body("").await)
                    .map_err(|_| ErrorCode::ConnectionReadTimeout)
                    .unwrap())
            }
        }
}