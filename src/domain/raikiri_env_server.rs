use std::{convert::Infallible, fmt::Display, net::SocketAddr, str::FromStr, sync::Arc};

use async_trait::async_trait;
use futures::{stream, TryFutureExt};
use http::{Request, Response};
use http_body_util::{combinators::BoxBody, Full, StreamBody};
use hyper::{body::{Body, Bytes, Frame, Incoming}, server::conn::http1, service::service_fn};
use tokio::net::TcpListener;
use wasmtime_wasi_http::{bindings::http::types::ErrorCode, io::TokioIo};


use crate::{adapters::{cache::Cache, component_invoke, component_registry, raikirifs::ThreadSafeError, secret_storage}, default_event_handler, new_empty_cache, ComponentEvent, ComponentImports, Wasi};

use super::raikiri_env::RaikiriEnvironment;

#[async_trait]
pub trait RaikiriEnvironmentServer {
    async fn init_server<T>(&mut self, port: u16) -> Result<(), ThreadSafeError>;
    async fn handle_request<B>(
        &self,
        request: Request<B>,
    ) -> Result<Response<BoxBody<Bytes, ErrorCode>>, ThreadSafeError>
    where
        B: Body<Data = Bytes, Error = hyper::Error> + Send + Sync + 'static;
    async fn response_body<E, T>(body: T) -> BoxBody<Bytes, E>
    where
        E: Send + Sync + 'static,
        T: ToString + Send;
}

#[async_trait]
impl RaikiriEnvironmentServer for RaikiriEnvironment {
    
    async fn init_server<T>(&mut self, port: u16) -> Result<(), ThreadSafeError> {
        self.port = Some(port);

        println!("Registering components...");
        self.component_registry = Some(component_registry::build_registry().await?);
        self.secrets_cache = Some(new_empty_cache());
        println!("Successfully registered components");

        Ok(())
    }

    async fn handle_request<B>(
        &self,
        request: Request<B>,
    ) -> Result<Response<BoxBody<Bytes, ErrorCode>>, Box<dyn std::error::Error + Send + Sync + 'static>>
    where
        B: Body<Data = Bytes, Error = hyper::Error> + Send + Sync + 'static,
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
                let secrets_entry = &self
                    .secrets_cache.clone().unwrap()
                    .get_entry_by_key_async_build(username_component_name.clone(), async {
                        secret_storage::get_component_secrets(username_component_name.clone())
                            .await
                            .unwrap_or_else(|_| Vec::new())
                    })
                    .await;
                let secrets = secrets_entry.read().await;
                let component_imports = ComponentImports {
                    call_stack: Vec::new(),
                    component_registry: self.component_registry.clone().unwrap(),
                    event_sender: tx,
                    secrets_cache: self.secrets_cache.clone().unwrap(),
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
                    .body(Self::response_body("").await)
                    .map_err(|_| ErrorCode::ConnectionReadTimeout)
                    .unwrap())
            }
        }
    }
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
}

pub async fn run_server(server: &RaikiriEnvironment) -> Result<(), ThreadSafeError> {
    let addr = SocketAddr::from(([127, 0, 0, 1], server.port.unwrap()));
    let listener = TcpListener::bind(addr).await?;
    loop {
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);
        let this = server.clone();

        tokio::task::spawn(async move {
            if let Err(err) = http1::Builder::new()
                .serve_connection(io, service_fn(|req| hello::<Incoming>(this.clone(), req)))
                .await
            {
                eprintln!("Error serving connection: {:?}", err);
            }
        });
    }
}

async fn hello<B>(_self: RaikiriEnvironment, request: Request<hyper::body::Incoming>) ->
    Result<Response<BoxBody<Bytes, ErrorCode>>, Box<dyn std::error::Error + Send + Sync + 'static>>
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
                    .secrets_cache.clone().unwrap()
                    .get_entry_by_key_async_build(username_component_name.clone(), async {
                        secret_storage::get_component_secrets(username_component_name.clone())
                            .await
                            .unwrap_or_else(|_| Vec::new())
                    })
                    .await;
                let secrets = secrets_entry.read().await;
                let component_imports = ComponentImports {
                    call_stack: Vec::new(),
                    component_registry: _self.component_registry.clone().unwrap(),
                    event_sender: tx,
                    secrets_cache: _self.secrets_cache.clone().unwrap(),
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