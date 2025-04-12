use std::{net::SocketAddr, sync::Arc};

use crate::{adapters::{component_imports::ComponentImports, raikirifs::ThreadSafeError, secret_storage, wasi_view::Wasi
}, domain::{raikiri_env::RaikiriEnvironment, raikiri_env_invoke::RaikiriEnvironmentInvoke}};

use futures::stream;
use http::{Request, Response};
use http_body_util::{combinators::BoxBody, BodyExt, StreamBody};
use hyper::{
    body::{Body, Bytes, Frame},
    server::conn::http1,
    service::service_fn,
};
use crate::domain::raikiri_env_component::RaikiriComponentStorage;
use tokio::net::TcpListener;
use wasmtime_wasi_http::bindings::http::types::ErrorCode;
use wasmtime_wasi_http::io::TokioIo;

#[derive(Clone)]
pub struct RaikiriServer {
    environment: RaikiriEnvironment,
    listener: Arc<TcpListener>
}
impl RaikiriServer {
    pub async fn new(
        environment: RaikiriEnvironment,
        port: u16,
    ) -> Result<Self, ThreadSafeError> {
        
        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        let listener = Arc::new(TcpListener::bind(addr).await?);
        println!("Raikiri server listening at port {port}");

        Ok(Self {
            environment,
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
            "Put-Component" => {
                let component_name = request.headers().get("Component-Id").unwrap()
                    .to_str().unwrap().to_string();
                let component_bytes = BoxBody::new(request.into_body()).collect().await.unwrap().to_bytes().to_vec();
                self.environment.add_component(self.environment.username.clone(), component_name, component_bytes).await.unwrap();
                Ok(Response::builder()
                    .status(200)
                    .body(Self::response_body("").await)
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

                
                let secrets_entry = &self.environment
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
                    environment: self.environment.clone(),
                };
                let response = self.environment.invoke_component(
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

    #[allow(dead_code)]
    pub async fn response_body_bytes<E>(body: Vec<u8>) -> BoxBody<Bytes, E>
    where
        E: Send + Sync + 'static,
    {
        BoxBody::new(StreamBody::new(stream::iter(
            body.chunks(16 * 1024)
                .map(|chunk| Ok::<_, E>(Frame::data(Bytes::copy_from_slice(chunk))))
                .collect::<Vec<_>>(),
        )))
    }
}

#[cfg(test)]
mod tests {

    use anyhow::Ok;
    use http::{Request, StatusCode};
    use http_body_util::BodyExt;

    use crate::{domain::{raikiri_env::RaikiriEnvironment, raikiri_env_fs::RaikiriEnvironmentFS}, server::RaikiriServer};

    impl Drop for RaikiriServer {
        fn drop(&mut self) {
            std::fs::remove_dir_all(self.environment.clone().fs_root).unwrap();
        }
    }

    #[tokio::test]
    async fn test_start_server() -> Result<(), wasmtime::Error> {

        let tmp_path = "/tmp/raikiri-0";
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
            .header("Component-Id", "test404.hello")
            .body(RaikiriServer::response_body("Hello World").await)
            .unwrap();

        let res = server.handle_request(request).await;

        assert_eq!(res.unwrap().status(), StatusCode::NOT_FOUND);

        Ok(())
    }

    #[tokio::test]
    async fn test_invoke_api_proxy() -> Result<(), wasmtime::Error> {

        let tmp_path = "/tmp/raikiri-1";
        tokio::fs::create_dir_all(tmp_path).await.unwrap();

        let environment = RaikiriEnvironment::new()
            .with_username("test".to_string())
            .with_fs_root(tmp_path.to_string());
        environment.setup_fs().await.unwrap();

        let server = RaikiriServer::new(environment, 0)
            .await
            .unwrap();

        // put component
        let component = tokio::fs::read(test_programs_artifacts::API_PROXY_COMPONENT).await.unwrap();
        let body = RaikiriServer::response_body_bytes(component).await;
        let request = Request::builder()
            .uri("/")
            .method("POST")
            .header("Platform-Command", "Put-Component")
            .header("Component-Id", "hello")
            .body(body)
            .unwrap();

        let res = server.handle_request(request).await;

        assert_eq!(res.unwrap().status(), StatusCode::OK);

        // send command to invoke component
        let request = Request::builder()
            .uri("https://localhost:8080")
            .method("GET")
            .header("Platform-Command", "Invoke-Component")
            .header("Component-Id", "test.hello")
            .header("Host", "localhost:8080")
            .body(RaikiriServer::response_body("").await)
            .unwrap();

        let res = server.handle_request(request).await;
        let (parts, body) = res.unwrap().into_parts();

        let body = body.collect().await.unwrap();
        let body = String::from_utf8(body.to_bytes().to_vec()).unwrap();

        assert_eq!(parts.status, StatusCode::OK);
        assert_eq!(body, "hello, world!");

        Ok(())
    }

    #[tokio::test]
    async fn test_invoke_hello() -> Result<(), wasmtime::Error> {

        let tmp_path = "/tmp/raikiri-2";
        tokio::fs::create_dir_all(tmp_path).await.unwrap();

        let environment = RaikiriEnvironment::new()
            .with_username("test".to_string())
            .with_fs_root(tmp_path.to_string());
        environment.setup_fs().await.unwrap();

        let server = RaikiriServer::new(environment, 0)
            .await
            .unwrap();

        // put component
        let component = tokio::fs::read(test_programs_artifacts::API_RAIKIRI_HELLO_COMPONENT).await.unwrap();
        let body = RaikiriServer::response_body_bytes(component).await;
        let request = Request::builder()
            .uri("/")
            .method("POST")
            .header("Platform-Command", "Put-Component")
            .header("Component-Id", "hello")
            .body(body)
            .unwrap();

        let res = server.handle_request(request).await;

        assert_eq!(res.unwrap().status(), StatusCode::OK);

        // send command to invoke component
        let request = Request::builder()
            .uri("https://localhost:8080")
            .method("GET")
            .header("Platform-Command", "Invoke-Component")
            .header("Component-Id", "test.hello")
            .header("Host", "localhost:8080")
            .body(RaikiriServer::response_body("").await)
            .unwrap();

        let res = server.handle_request(request).await;
        let (parts, body) = res.unwrap().into_parts();

        let body = body.collect().await.unwrap();
        let body = String::from_utf8(body.to_bytes().to_vec()).unwrap();

        assert_eq!(parts.status, StatusCode::OK);
        assert_eq!(body, "Hello World!");

        Ok(())
    }
}
