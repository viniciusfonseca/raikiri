use std::{convert::Infallible, net::SocketAddr, sync::Arc};

use crate::adapters::{cache::{new_empty_cache, Cache}, component_events::default_event_handler, raikirifs::ThreadSafeError, secret_storage};

use http::{Request, Response};
use http_body_util::{combinators::BoxBody, BodyExt};
use hyper::{body::Bytes, server::conn::http1, service::service_fn};
use tokio::net::TcpListener;
use wasmtime::component::Component;
use wasmtime_wasi_http::io::TokioIo;
use wasmtime_wasi_http::bindings::http::types::ErrorCode;

use crate::{adapters::{component_events::ComponentEvent, component_imports::ComponentImports, component_invoke, component_registry, wasi_view::Wasi}, types::InvokeRequest};

#[derive(Clone)]
pub struct RaikiriServer {
    component_registry: Cache<String, Component>,
    secrets_cache: Cache<String, Vec<(String, String)>>,
    listener: Arc<TcpListener>
}
impl RaikiriServer {
    
    pub async fn new(port: String) -> Result<Self, ThreadSafeError> {
        
        let addr = SocketAddr::from(([127, 0, 0, 1], port.parse()?));
        let listener = Arc::new(TcpListener::bind(addr).await?);
        println!("Raikiri server listening at port {port}");
        
        println!("Registering components...");
        let component_registry = component_registry::build_registry().await?;
        let secrets_cache = new_empty_cache();
        println!("Successfully registered components");
        
        Ok(Self {
            component_registry,
            secrets_cache,
            listener
        })
    }
    
    pub async fn run(&self) -> Result<(), ThreadSafeError> {
        loop {
            let (stream, _) = self.listener.accept().await?;
            let io = TokioIo::new(stream);
            let this = self.clone();
            
            tokio::task::spawn(async move {
                if let Err(err) = http1::Builder::new()
                .serve_connection(io, service_fn(|req| this.handle_request(req))).await
                {
                    eprintln!("Error serving connection: {:?}", err);
                }
            });
        }
    }
    
    pub async fn handle_request(&self, request: Request<hyper::body::Incoming>) -> Result<Response<BoxBody<Bytes, ErrorCode>>, Infallible> {
        let invoke_request = serde_json::from_slice::<InvokeRequest>(&request.into_body().collect().await.unwrap().to_bytes().to_vec()).unwrap();
        let username_component_name = invoke_request.username_component_name.clone();
    
        let (tx, mut rx) = tokio::sync::mpsc::channel::<ComponentEvent>(0xFFFF);
        tokio::spawn(async move {
            while let Some(message) = rx.recv().await {
                default_event_handler(message)
            }
        });
        let secrets_entry = &self.secrets_cache.get_entry_by_key_async_build(username_component_name.clone(), async {
            secret_storage::get_component_secrets(username_component_name.clone()).await.unwrap_or_else(|_| Vec::new())
        }).await;
        let secrets = secrets_entry.read().await;
        let component_imports = ComponentImports {
            call_stack: Vec::new(),
            component_registry: self.component_registry,
            event_sender: tx,
            secrets_cache: self.secrets_cache
        };
        let response = component_invoke::invoke_component(username_component_name.clone(), invoke_request.into(), Wasi::new(component_imports, secrets.to_vec())).await.unwrap();
    
        let (parts, body) = response.resp.into_parts();
        Ok(hyper::Response::from_parts(parts, body))
    }
    
}

#[cfg(test)]
mod tests {
    use anyhow::Ok;

    use super::*;

    #[tokio::test]
    async fn test_start_server() -> Result<(), wasmtime::Error> {

        Ok(())
    }
}