use std::{collections::HashMap, sync::Arc};

use adapters::{component_events::ComponentEvent, component_invoke, component_storage, setup_app_dir::setup_app_dir};
use clap::{Parser, Subcommand};
use http_body_util::BodyExt;
// use serde_json::{Map, Value};
use server::start_server;
use tokio::sync::RwLock;
use types::InvokeRequest;
use wasmtime::component::Component;

mod server;
mod adapters;
mod types;

#[derive(Debug, Parser)]
#[command(name = "raikiri")]
struct Cli {
    #[command(subcommand)]
    command: Commands
}

#[derive(Debug, Subcommand)]
enum Commands {

    #[command(arg_required_else_help = true)]
    Wasm {
        #[command(subcommand)]
        command: WasmSubcommand
    },
    #[command(arg_required_else_help = true)]
    Server {
        #[command(subcommand)]
        command: ServerSubcommand
    },
    #[command(arg_required_else_help = true)]
    Component {
        #[command(subcommand)]
        command: ComponentSubcommand
    }
}

#[derive(Debug, Clone, Subcommand)]
enum WasmSubcommand {
    Run {
        path: String
    }
}

#[derive(Debug, Clone, Subcommand)]
enum ServerSubcommand {
    Start {
        #[arg(short, long)]
        port: String
    }
}

#[derive(Debug, Clone, Subcommand)]
enum ComponentSubcommand {
    Add {
        #[arg(short, long)]
        name: String,
        #[arg(short, long)]
        path: String
    },
    Run {
        #[arg(short, long)]
        request: String,
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let username = whoami::username();
    setup_app_dir()?;
    match Cli::parse().command {
        Commands::Wasm { command } => {
            match command {
                WasmSubcommand::Run { path } => {
                    println!("running wasm: {path}")
                }
            }
        },
        Commands::Server { command } => {
            match command {
                ServerSubcommand::Start { port } => {
                    println!("starting Raikiri server at port: {port}");
                    start_server(port).await?;
                }
            }
        },
        Commands::Component { command } => {
            match command {
                ComponentSubcommand::Add { name, path } => {
                    let username_component_name = format!("{username}.{name}");
                    component_storage::add_component(username, name.clone(), path).await?;
                    println!("Successfully added component {username_component_name}");
                },
                ComponentSubcommand::Run { request } => {
                    let request = serde_json::from_str::<InvokeRequest>(&request)?;
                    let username_component_name = request.username_component_name.clone();
                    let (tx, mut rx) = tokio::sync::mpsc::channel::<ComponentEvent>(0xFFFF);
                    tokio::spawn(async move {
                        while let Some(message) = rx.recv().await {
                            match message {
                                ComponentEvent::Stdout { stdout, username_component_name } =>
                                    println!("Stdout from {username_component_name}: {}", String::from_utf8(stdout.contents().to_vec()).unwrap()),
                            }
                        }
                    });
                    let component_registry = Arc::new(RwLock::new(HashMap::<String, Component>::new()));
                    let response = component_invoke::invoke_component(username_component_name.clone(), request.into(), Vec::new(), component_registry, tx).await?;
                    println!("Successfully invoked {username_component_name}");
                    let resp_body = BodyExt::collect(response.resp.into_body()).await?.to_bytes().to_vec();
                    println!("Response: {}", String::from_utf8(resp_body)?);
                }
            }
        }
    }
    Ok(())
}

