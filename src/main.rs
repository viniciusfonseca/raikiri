
use adapters::{
    module_events::ModuleEvent, module_invoke, module_storage, setup_app_dir::setup_app_dir
};
use clap::{Parser, Subcommand};
use server::start_server;

mod server;
mod adapters;

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
    Module {
        #[command(subcommand)]
        command: ModuleSubcommand
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
enum ModuleSubcommand {
    Add {
        #[arg(short, long)]
        name: String,
        #[arg(short, long)]
        path: String
    },
    Run {
        #[arg(short, long)]
        name: String,
        #[arg(short, long)]
        params: Vec<u8>,
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
                    start_server();
                }
            }
        },
        Commands::Module { command } => {
            match command {
                ModuleSubcommand::Add { name, path } => {
                    let username_module_name = format!("{username}.{name}");
                    module_storage::add_module(username, name.clone(), path).await?;
                    println!("Successfully added module {username_module_name}");
                },
                ModuleSubcommand::Run { name, params } => {
                    let username_module_name = format!("{username}.{name}");
                    let (tx, mut rx) = tokio::sync::mpsc::channel::<ModuleEvent>(0xFFFF);
                    tokio::spawn(async move {
                        while let Some(message) = rx.recv().await {
                            match message {
                                ModuleEvent::Stdout { stdout, username_module_name } =>
                                    println!("Stdout from {username_module_name}: {}", String::from_utf8(stdout.contents().to_vec()).unwrap()),
                            }
                        }
                    });
                    let response = module_invoke::invoke_wasm_module(username_module_name.clone(), params, Vec::new(), tx).await?;
                    println!("Successfully invoked {username_module_name}");
                    println!("Response: {}", String::from_utf8(response.body)?);
                }
            }
        }
    }
    Ok(())
}

