use std::net::SocketAddr;

use async_trait::async_trait;
use futures::stream;
use http::{Request, Response};
use http_body_util::{combinators::BoxBody, BodyExt, StreamBody};
use hyper::{body::{Body, Bytes, Frame, Incoming}, server::conn::http1, service::service_fn};
use tokio::net::TcpListener;
use wasmtime_wasi_http::{bindings::http::types::ErrorCode, io::TokioIo};


use crate::{ComponentImports, Wasi};

use super::{raikiri_env::{RaikiriEnvironment, ThreadSafeError}, raikiri_env_component::RaikiriComponentStorage, raikiri_env_invoke::RaikiriEnvironmentInvoke, raikiri_env_secrets::RaikiriEnvironmentSecrets};

#[async_trait]
pub trait RaikiriEnvironmentServer {
    async fn response_body<E, T>(body: T) -> BoxBody<Bytes, E>
    where
        E: Send + Sync + 'static,
        T: ToString + Send;
    async fn response_body_bytes<E>(body: Vec<u8>) -> BoxBody<Bytes, E>
    where
        E: Send + Sync + 'static;
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

    async fn response_body_bytes<E>(body: Vec<u8>) -> BoxBody<Bytes, E>
    where
        E: Send + Sync + 'static,
    {
        BoxBody::new(StreamBody::new(stream::iter(
            body.chunks(16 * 1024)
                .map(|chunk| Ok::<_, E>(Frame::data(Bytes::copy_from_slice(chunk))))
                .collect::<Vec<_>>(),
        )))
    }

    async fn run_server(&self) -> Result<(), ThreadSafeError> {
        let self = self.clone();
        tokio::spawn(async move {
            let addr = SocketAddr::from(([127, 0, 0, 1], self.port));
            let listener = TcpListener::bind(addr).await.unwrap();
            loop {
                let (stream, _) = listener.accept().await.unwrap(); 
                let io = TokioIo::new(stream);
                let self = self.clone();
        
                tokio::task::spawn(async move {
                    if let Err(err) = http1::Builder::new()
                        .serve_connection(io, service_fn(|req| handle_request::<Incoming>(&self, req)))
                        .await
                    {
                        eprintln!("Error serving connection: {:?}", err);
                    }
                });
            }
        });
        Ok(())
    }
}

pub async fn handle_request<B>(_self: &RaikiriEnvironment, request: Request<B>) ->
    Result<Response<BoxBody<Bytes, ErrorCode>>, ThreadSafeError>
    where
        B: Body<Data = Bytes, Error = hyper::Error> + Send + Sync + 'static
{
    let command = request.headers()
        .get("Platform-Command").unwrap()
        .to_str().unwrap();

    match command {
        "Put-Component" => {
            let component_name = request.headers().get("Component-Id").unwrap()
                .to_str().unwrap().to_string();
            let component_bytes = BoxBody::new(request.into_body()).collect().await.unwrap().to_bytes().to_vec();
            _self.add_component(_self.username.clone(), component_name, component_bytes).await.unwrap();
            Ok(Response::builder()
                .status(200)
                .body(RaikiriEnvironment::response_body("").await)
                .map_err(|_| ErrorCode::ConnectionReadTimeout)
                .unwrap())
        }
        "Invoke-Component" => {
            let username_component_name = request
                .headers()
                .get("Component-Id")
                .unwrap()
                .to_str()
                .unwrap()
                .to_string();
            
            let secrets_entry = _self.secrets_cache
                .get_entry_by_key_async_build(username_component_name.clone(), async {
                    // split username_component_name into username and component name by dot
                    let (username, component_name) = username_component_name.split_once('.').unwrap();
                    _self.get_component_secrets(username.to_string(), component_name.to_string())
                        .await
                        .unwrap_or_else(|_| Vec::new())
                })
                .await;
            let secrets = secrets_entry.read().await;
            let component_imports = ComponentImports {
                call_stack: Vec::new(),
                environment: _self.clone(),
                db_connections: Default::default()
            };
            let response = _self.invoke_component(
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

#[cfg(test)]
mod tests {

    use anyhow::{Ok, Result};
    use http::{Request, StatusCode};
    use http_body_util::BodyExt;

    use crate::domain::{raikiri_env::RaikiriEnvironment, raikiri_env_fs::RaikiriEnvironmentFS, raikiri_env_server::{handle_request, RaikiriEnvironmentServer}, tests::{create_test_env, make_put_component_request}};

    impl Drop for RaikiriEnvironment {
        fn drop(&mut self) {
            _ = std::fs::remove_dir_all(self.fs_root.clone());
        }
    }

    #[tokio::test]
    async fn test_start_server() -> Result<()> {

        let environment = create_test_env();
        environment.setup_fs().await.unwrap();

        let request = Request::builder()
            .uri("/")
            .method("GET")
            .header("Platform-Command", "Invoke-Component")
            .header("Component-Id", "test404.hello")
            .body(RaikiriEnvironment::response_body("Hello World").await)
            .unwrap();

        let res = handle_request(&environment, request).await;

        assert_eq!(res.unwrap().status(), StatusCode::NOT_FOUND);

        Ok(())
    }

    #[tokio::test]
    async fn test_invoke_api_proxy() -> Result<(), wasmtime::Error> {

        let environment = create_test_env();
        environment.setup_fs().await.unwrap();

        let req = make_put_component_request(test_programs_artifacts::API_PROXY_COMPONENT).await;
        let res = handle_request(&environment, req).await;

        assert_eq!(res.unwrap().status(), StatusCode::OK);

        // send command to invoke component
        let request = Request::builder()
            .uri("https://localhost:8080")
            .method("GET")
            .header("Platform-Command", "Invoke-Component")
            .header("Component-Id", "test.hello")
            .header("Host", "localhost:8080")
            .body(RaikiriEnvironment::response_body("").await)
            .unwrap();

        let res = handle_request(&environment, request).await;
        let (parts, body) = res.unwrap().into_parts();

        let body = body.collect().await.unwrap();
        let body = String::from_utf8(body.to_bytes().to_vec()).unwrap();

        assert_eq!(parts.status, StatusCode::OK);
        assert_eq!(body, "hello, world!");

        Ok(())
    }

    #[tokio::test]
    async fn test_invoke_hello() -> Result<(), wasmtime::Error> {

        let environment = create_test_env();
        environment.setup_fs().await.unwrap();

        // put component
        let req = make_put_component_request(test_programs_artifacts::API_RAIKIRI_HELLO_COMPONENT).await;
        let res = handle_request(&environment, req).await;

        assert_eq!(res.unwrap().status(), StatusCode::OK);

        // send command to invoke component
        let request = Request::builder()
            .uri("https://localhost:8080")
            .method("GET")
            .header("Platform-Command", "Invoke-Component")
            .header("Component-Id", "test.hello")
            .header("Host", "localhost:8080")
            .body(RaikiriEnvironment::response_body("").await)
            .unwrap();

        let res = handle_request(&environment, request).await;
        let (parts, body) = res.unwrap().into_parts();

        let body = body.collect().await.unwrap();
        let body = String::from_utf8(body.to_bytes().to_vec()).unwrap();

        assert_eq!(parts.status, StatusCode::OK);
        assert_eq!(body, "Hello World!");

        Ok(())
    }
}