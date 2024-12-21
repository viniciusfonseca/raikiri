use std::{convert::Infallible, net::SocketAddr};

use crate::adapters::{cache::{new_empty_cache, Cache}, component_events::default_event_handler, raikirifs::ThreadSafeError, secret_storage};

use http::{Request, Response};
use http_body_util::{combinators::BoxBody, BodyExt};
use hyper::{body::Bytes, server::conn::http1, service::service_fn};
use tokio::net::TcpListener;
use wasmtime::component::Component;
use wasmtime_wasi_http::io::TokioIo;
use wasmtime_wasi_http::bindings::http::types::ErrorCode;

use crate::{adapters::{component_events::ComponentEvent, component_imports::ComponentImports, component_invoke, component_registry, wasi_view::Wasi}, types::InvokeRequest};

async fn handle_request(request: Request<hyper::body::Incoming>, component_registry: Cache<String, Component>, secrets_cache: Cache<String, Vec<(String, String)>>) -> Result<Response<BoxBody<Bytes, ErrorCode>>, Infallible> {
    let invoke_request = serde_json::from_slice::<InvokeRequest>(&request.into_body().collect().await.unwrap().to_bytes().to_vec()).unwrap();
    let username_component_name = invoke_request.username_component_name.clone();

    let (tx, mut rx) = tokio::sync::mpsc::channel::<ComponentEvent>(0xFFFF);
    tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            default_event_handler(message)
        }
    });
    let secrets_entry = &secrets_cache.get_entry_by_key_async_build(username_component_name.clone(), async {
        secret_storage::get_component_secrets(username_component_name.clone()).await.unwrap_or_else(|_| Vec::new())
    }).await;
    let secrets = secrets_entry.read().await;
    let component_imports = ComponentImports {
        call_stack: Vec::new(),
        component_registry,
        event_sender: tx,
        secrets_cache
    };
    let response = component_invoke::invoke_component(username_component_name.clone(), invoke_request.into(), Wasi::new(component_imports, secrets.to_vec())).await.unwrap();

    let (parts, body) = response.resp.into_parts();
    Ok(hyper::Response::from_parts(parts, body))
}

pub async fn start_server(port: String) -> Result<(), ThreadSafeError> {
    let addr = SocketAddr::from(([127, 0, 0, 1], port.parse()?));

    let listener = TcpListener::bind(addr).await?;

    println!("Raikiri server listening at port {port}");

    println!("Registering components...");

    let component_registry = component_registry::build_registry().await?;
    let secrets_cache = new_empty_cache();

    println!("Successfully registered components");

    // We start a loop to continuously accept incoming connections
    loop {
        let (stream, _) = listener.accept().await?;
        let component_registry = component_registry.clone();
        let secrets_cache = secrets_cache.clone();
        // Use an adapter to access something implementing `tokio::io` traits as if they implement
        // `hyper::rt` IO traits.
        let io = TokioIo::new(stream);

        // Spawn a tokio task to serve multiple connections concurrently
        tokio::task::spawn(async move {
            // Finally, we bind the incoming connection to our `hello` service
            if let Err(err) = http1::Builder::new()
                // `service_fn` converts our function in a `Service`
                .serve_connection(io, service_fn(|req| handle_request(req, component_registry.clone(), secrets_cache.clone())))
                .await
            {
                eprintln!("Error serving connection: {:?}", err);
            }
        });
    }
}
