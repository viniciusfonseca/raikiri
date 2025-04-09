use std::{net::SocketAddr, sync::Arc};

use crate::adapters::{
    cache::{new_empty_cache, Cache}, component_events::{default_event_handler, ComponentEvent}, component_imports::ComponentImports, component_invoke, component_registry, raikirifs::ThreadSafeError, secret_storage, wasi_view::Wasi
};

use futures::stream;
use http::{Request, Response};
use http_body_util::{combinators::BoxBody, StreamBody};
use hyper::{
    body::{Body, Bytes, Frame},
    server::conn::http1,
    service::service_fn,
};
use raikiri::raikiri_env::RaikiriEnvironment;
use tokio::net::TcpListener;
use wasmtime::component::Component;
use wasmtime_wasi_http::bindings::http::types::ErrorCode;
use wasmtime_wasi_http::io::TokioIo;

#[derive(Clone)]
pub struct RaikiriServer {
    environment: RaikiriEnvironment,
    component_registry: Cache<String, Component>,
    secrets_cache: Cache<String, Vec<(String, String)>>,
    listener: Arc<TcpListener>,
}
impl RaikiriServer {
    pub async fn new(
        environment: RaikiriEnvironment,
        port: u16,
    ) -> Result<Self, ThreadSafeError> {
        
        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        let listener = Arc::new(TcpListener::bind(addr).await?);
        println!("Raikiri server listening at port {port}");

        println!("Registering components...");
        let component_registry = component_registry::build_registry().await?;
        let secrets_cache = new_empty_cache();
        println!("Successfully registered components");

        Ok(Self {
            environment,
            component_registry,
            secrets_cache,
            listener,
        })
    }

    pub async fn run(&self) -> Result<(), ThreadSafeError> {
        loop {
            let (stream, _) = self.listener.accept().await?;
            let io = TokioIo::new(stream);
            let this = self.clone();

            tokio::task::spawn(async move {
                if let Err(err) = http1::Builder::new()
                    .serve_connection(io, service_fn(|req| this.handle_request(req)))
                    .await
                {
                    eprintln!("Error serving connection: {:?}", err);
                }
            });
        }
    }

    pub async fn handle_request<B>(
        &self,
        request: Request<B>,
    ) -> Result<Response<BoxBody<Bytes, ErrorCode>>, ThreadSafeError>
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
                    component_registry: self.component_registry.clone(),
                    event_sender: tx,
                    secrets_cache: self.secrets_cache.clone(),
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

    pub async fn response_body<E, T>(body: T) -> BoxBody<Bytes, E>
    where
        E: Send + Sync + 'static,
        T: ToString,
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use anyhow::Ok;
    use http::{Request, StatusCode};
    use raikiri::raikiri_env::RaikiriEnvironment;

    use crate::server::RaikiriServer;
    use raikiri::domain::raikiri_env_fs::RaikiriEnvironmentFS;

    impl Drop for RaikiriServer {
        fn drop(&mut self) {
            std::fs::remove_dir_all(self.environment.clone().fs_root).unwrap();
        }
    }

    #[tokio::test]
    async fn test_start_server() -> Result<(), wasmtime::Error> {

        let tmp_path = "/tmp/raikiri";
        tokio::fs::create_dir_all(tmp_path).await.unwrap();

        let environment = RaikiriEnvironment::new()
            .with_username("test".to_string())
            .with_fs_root(tmp_path.to_string());
        environment.setup_fs().await.unwrap();

        let server = RaikiriServer::new(environment, 0)
            .await
            .unwrap();

        let request = Request::builder()
            .uri("/")
            .method("GET")
            .header("Platform-Command", "Invoke-Component")
            .header("Component-Id", "test.hello")
            .body(RaikiriServer::response_body("Hello World").await)
            .unwrap();

        let res = server.handle_request(request).await;

        assert_eq!(res.unwrap().status(), StatusCode::NOT_FOUND);

        Ok(())
    }
}
