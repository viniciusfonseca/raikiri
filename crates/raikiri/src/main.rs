use adapters::{cache::new_empty_cache, component_imports::ComponentImports, component_storage, raikirifs::{self, init, ThreadSafeError}, secret_storage, wasi_view::Wasi};
use clap::{Parser, Subcommand};
use domain::{raikiri_env::RaikiriEnvironment, raikiri_env_invoke::RaikiriEnvironmentInvoke, raikiri_env_server::RaikiriEnvironmentServer};
use http_body_util::BodyExt;
use types::InvokeRequest;

mod adapters;
mod types;
mod sdk;
mod domain;

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
    #[command(arg_required_else_help = true)]
    Cloud {
        #[command(subcommand)]
        command: CloudSubcommand
    },
    UpdateCryptoKey {
        #[arg(short, long)]
        path: String
    },
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
        conf: Option<String>,
    },
    UpdateSecret {
        #[arg(short, long)]
        component_name: String,
        #[arg(short, long)]
        secrets_path: String,
    }
}

#[derive(Debug, Clone, Subcommand)]
enum CloudSubcommand {
    StoreToken {
        token: String
    },
    UploadComponent {
        #[arg(short, long)]
        name: String,
        #[arg(short, long)]
        path: String
    },
    CreateApiGateway {
        #[arg(short, long)]
        path: String,
        #[arg(short, long)]
        version: String
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
                    let env = RaikiriEnvironment::new();
                    env.run_server().await?;
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
                ComponentSubcommand::Run { conf } => {
                    let conf_file = adapters::conf_file::ConfFile::build()?;
                    let conf = match conf {
                        Some(conf) => conf,
                        None => conf_file.run_confs.keys().next().unwrap().to_string()
                    };
                    let conf = conf_file.run_confs.get(&conf).unwrap();
                    let request = InvokeRequest::new(conf.component.clone(), conf.method.clone(), conf.headers.clone(), conf.body.as_bytes().to_vec());
                    let username_component_name = request.username_component_name.clone();
                    let environment = RaikiriEnvironment::new();
                    let component_imports = ComponentImports {
                        call_stack: Vec::new(),
                        environment: RaikiriEnvironment::new(),
                    };
                    let secrets = secret_storage::get_component_secrets(username_component_name.clone()).await?;
                    let response = environment.invoke_component(username_component_name.clone(), request.into(), Wasi::new(component_imports, secrets)).await?;
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
                    println!("Updating secret for component {username_component_name}");
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
        },
        Commands::Cloud { command } => {
            match command {
                CloudSubcommand::StoreToken { token } => {
                    raikirifs::write_file(".cloud-token".to_string(), token.into_bytes().to_vec()).await?;
                    println!("Successfully stored token");
                }
                CloudSubcommand::UploadComponent { name, path } => {
                    let username_component_name = format!("{username}.{name}");
                    sdk::upload_component(username, name.clone(), path).await?;
                    println!("Successfully uploaded component {username_component_name}");
                },
                CloudSubcommand::CreateApiGateway { path, version } => {
                    let yml_bytes = tokio::fs::read(path).await?;
                    let version = version.parse::<i32>()?;
                    sdk::create_api_gateway(yml_bytes, version).await?;
                    println!("Successfully created api gateway");
                }
            }
        }
    }
    Ok(())
}

