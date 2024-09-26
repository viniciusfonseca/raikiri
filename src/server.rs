use std::{collections::HashMap, convert::Infallible, net::SocketAddr, sync::{Arc, RwLock}};

use http::{Request, Response};
use http_body_util::{combinators::BoxBody, BodyExt};
use hyper::{body::Bytes, server::conn::http1, service::service_fn};
use tokio::net::TcpListener;
use wasmtime::component::Component;
use wasmtime_wasi_http::io::TokioIo;
use wasmtime_wasi_http::bindings::http::types::ErrorCode;

use crate::{adapters::{component_events::ComponentEvent, component_invoke}, types::InvokeRequest};

async fn handle_request(request: Request<hyper::body::Incoming>) -> Result<Response<BoxBody<Bytes, ErrorCode>>, Infallible> {
    let invoke_request = serde_json::from_slice::<InvokeRequest>(&request.into_body().collect().await.unwrap().to_bytes().to_vec()).unwrap();
    let username_component_name = invoke_request.username_component_name.clone();

    let (tx, mut rx) = tokio::sync::mpsc::channel::<ComponentEvent>(0xFFFF);
    tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            match message {
                ComponentEvent::Stdout { stdout, username_component_name } =>
                    println!("Stdout from {username_component_name}: {}", String::from_utf8(stdout.contents().to_vec()).unwrap()),
            }
        }
    });

    let response = component_invoke::invoke_component(username_component_name.clone(), invoke_request.into(), Vec::new(), tx).await.unwrap();

    let (parts, body) = response.resp.into_parts();
    Ok(hyper::Response::from_parts(parts, body))
}

pub async fn start_server() -> Result<(), Box<dyn std::error::Error>> {
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    let listener = TcpListener::bind(addr).await?;

    let registered_components = Arc::new(RwLock::new(HashMap::<String, Component>::new()));

    // We start a loop to continuously accept incoming connections
    loop {
        let (stream, _) = listener.accept().await?;

        // Use an adapter to access something implementing `tokio::io` traits as if they implement
        // `hyper::rt` IO traits.
        let io = TokioIo::new(stream);

        // Spawn a tokio task to serve multiple connections concurrently
        tokio::task::spawn(async move {
            // Finally, we bind the incoming connection to our `hello` service
            if let Err(err) = http1::Builder::new()
                // `service_fn` converts our function in a `Service`
                .serve_connection(io, service_fn(handle_request))
                .await
            {
                eprintln!("Error serving connection: {:?}", err);
            }
        });
    }
}