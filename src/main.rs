use adapters::{cache::new_empty_cache, component_events::{default_event_handler, ComponentEvent}, component_imports::ComponentImports, component_invoke, component_storage, raikirifs::{init, ThreadSafeError}, secret_storage, wasi_view::Wasi};
use clap::{Parser, Subcommand};
use http_body_util::BodyExt;
use server::start_server;
use types::InvokeRequest;

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
    Server {
        #[command(subcommand)]
        command: ServerSubcommand
    },
    #[command(arg_required_else_help = true)]
    Component {
        #[command(subcommand)]
        command: ComponentSubcommand
    },
    UpdateCryptoKey {
        #[arg(short, long)]
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
    Remove {
        #[arg(short, long)]
        name: String,
    },
    Run {
        #[arg(short, long)]
        request: String,
    },
    UpdateSecret {
        #[arg(short, long)]
        component_name: String,
        #[arg(short, long)]
        secrets_path: String,
    }
}

#[tokio::main]
async fn main() -> Result<(), ThreadSafeError> {
    let username = whoami::username();
    init().await?;
    match Cli::parse().command {
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
                            default_event_handler(message)
                        }
                    });
                    let component_registry = new_empty_cache();
                    let component_imports = ComponentImports {
                        call_stack: Vec::new(),
                        component_registry,
                        event_sender: tx
                    };
                    let secrets = secret_storage::get_component_secrets(username_component_name.clone()).await?;
                    let response = component_invoke::invoke_component(username_component_name.clone(), request.into(), Wasi::new(component_imports, secrets)).await?;
                    println!("Successfully invoked {username_component_name}");
                    let resp_body = BodyExt::collect(response.resp.into_body()).await?.to_bytes().to_vec();
                    println!("Response: {}", String::from_utf8(resp_body)?);
                },
                ComponentSubcommand::Remove { name } => {
                    let username_component_name = format!("{username}.{name}");
                    component_storage::remove_component(username, name).await?;
                    println!("Successfully removed component {username_component_name}");
                },
                ComponentSubcommand::UpdateSecret { component_name, secrets_path } => {
                    let username_component_name = format!("{username}.{component_name}");
                    let secrets_content = tokio::fs::read(secrets_path).await?;
                    secret_storage::update_component_secrets(username_component_name.clone(), secrets_content).await?;
                    println!("Successfully updated secret for component {username_component_name}");
                }
            }
        },
        Commands::UpdateCryptoKey { path } => {
            let key_bytes = tokio::fs::read(path).await?;
            secret_storage::update_crypto_key(username, key_bytes).await?;
            println!("Successfully updated crypto key");
        }
    }
    Ok(())
}

