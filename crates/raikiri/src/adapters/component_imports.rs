use std::time::Duration;

use futures::stream;
use http::Response;
use http_body_util::{combinators::BoxBody, BodyExt, StreamBody};
use hyper::body::{Bytes, Frame};
use testcontainers::bollard::secret;
use wasmtime_wasi_http::types::{HostFutureIncomingResponse, IncomingResponse};

use crate::domain::{raikiri_env::RaikiriEnvironment, raikiri_env_db::{RaikiriDBConnectionKind, RaikiriEnvironmentDB}, raikiri_env_invoke::{build_response, RaikiriEnvironmentInvoke}, raikiri_env_secrets::RaikiriEnvironmentSecrets};

use super::{context::RaikiriContext, wasi_view::Wasi};

#[derive(Clone)]
pub struct ComponentImports {
    pub call_stack: Vec<String>,
    pub environment: RaikiriEnvironment,
}

impl RaikiriContext for ComponentImports {
    fn call_stack(&self) -> &Vec<String> {
        &self.call_stack
    }

    fn environment(&self) -> &RaikiriEnvironment {
        &self.environment
    }
    
    fn handle_http(&self, request: hyper::Request<wasmtime_wasi_http::body::HyperOutgoingBody>,
        config: wasmtime_wasi_http::types::OutgoingRequestConfig,
    ) -> wasmtime_wasi_http::HttpResult<wasmtime_wasi_http::types::HostFutureIncomingResponse> {
        match request.uri().host().unwrap() {
            "raikiri.components" => {
                let data = self.clone();
                let username_component_name = request.uri().path().replace("/", "");
                let future_handle = wasmtime_wasi::runtime::spawn(async move {
                    let mut request_builder = hyper::Request::builder()
                        .uri(request.uri());
                    for (key, value) in request.headers() {
                        request_builder = request_builder.header(key, value);
                    }
                    let body = request.into_body().collect().await.unwrap().to_bytes().to_vec();
                    let request = request_builder.body(BoxBody::new(StreamBody::new(stream::iter(
                        body.chunks(16 * 1024)
                            .map(|chunk| Ok::<_, hyper::Error>(Frame::data(Bytes::copy_from_slice(chunk))))
                            .collect::<Vec<_>>()
                    )))).unwrap();
                    let secrets_entry = &data.environment.secrets_cache.get_entry_by_key_async_build(username_component_name.clone(), async {
                        let (username, component_name) = username_component_name.split_once('.').unwrap();
                        data.environment.get_component_secrets(username.to_string(), component_name.to_string()).await.unwrap_or_else(|_| Vec::new())
                    }).await;
                    let secrets = secrets_entry.read().await;
                    let wasi = Wasi::new(data.clone(), secrets.to_vec());
                    Ok(data.environment.invoke_component(username_component_name, request, wasi).await)
                });
                Ok(HostFutureIncomingResponse::Pending(future_handle))
            }
            "raikiri.db" => {
                let data = self.clone();
                let future_handle = wasmtime_wasi::runtime::spawn(async move {
                    match request.uri().path() {
                        "/postgres_connection" => {
                            let caller = data.call_stack().last().unwrap();
                            let secrets_entry = &data.environment.secrets_cache.get_entry_by_key_async_build(caller.clone(), async {
                                data.environment.get_component_secrets(caller.to_string(), "secrets".to_string()).await.unwrap_or_else(|_| Vec::new())
                            }).await;
                            let secrets = secrets_entry.read().await;
                            let postgres_connection_string = &secrets.iter().find(|(key, _)| key == "POSTGRES_CONNECTION_STRING").unwrap().1;
                            let connection = data.environment.create_connection(RaikiriDBConnectionKind::POSTGRESQL, postgres_connection_string.as_str().as_bytes().to_vec()).await;
                            let connection_id = uuid::Uuid::new_v4().to_string();
                            data.environment.db_connections.insert(connection_id.clone(), connection);
                            Ok(Ok(build_response(200, &connection_id).await))
                        }
                        "/execute" => {
                            let connection_id = request.headers().get("Connection-Id").unwrap().to_str().unwrap();
                            let connection = data.environment.get_connection(connection_id.to_string()).await;
                            let body = request.into_body().collect().await.unwrap().to_bytes().to_vec();
                            let response = connection.execute_command(body).await.unwrap();
                            Ok(Ok(build_response(200, &String::from_utf8(response).unwrap()).await))
                        }
                        _ => Ok(Ok(build_response(404, "").await))
                    }
                });
                Ok(HostFutureIncomingResponse::Pending(future_handle))
            }
            _ => Ok(wasmtime_wasi_http::types::default_send_request(request, config))
        }
    }
}