use std::sync::Arc;

use chrono::DateTime;
use tokio::sync::Mutex;
use wasmtime::{Config, Engine};
use wasmtime_wasi::pipe::MemoryOutputPipe;

use crate::{adapters::{cache::Cache, conf_file::ConfFile}, domain::raikiri_env_component::RaikiriComponentStorage, new_empty_cache};

use super::{raikiri_env_component::ComponentRegistry, raikiri_env_db::RaikiriDBConnection};

#[derive(Clone)]
pub struct RaikiriEnvironment {
    pub fs_root: String,
    pub username: String,
    pub wasm_engine: Engine,
    pub component_registry: ComponentRegistry,
    pub secrets_cache: Cache<String, Vec<(String, String)>>,
    pub port: u16,
    pub conf_file: ConfFile,
    pub event_sender: tokio::sync::mpsc::Sender<ComponentEvent>,
    pub event_receiver: Arc<Mutex<tokio::sync::mpsc::Receiver<ComponentEvent>>>,
    pub event_handler: Option<fn(ComponentEvent) -> ()>,
    pub db_connections: scc::HashMap<String, Arc<dyn RaikiriDBConnection + Send + Sync>>
}

impl Default for RaikiriEnvironment {
    fn default() -> Self {
        Self::new()
    }
}

impl RaikiriEnvironment {
    pub fn new() -> Self {
        let mut config = Config::new();
        config.cache_config_load_default().expect("could not load default cache config");
        config.wasm_backtrace_details(wasmtime::WasmBacktraceDetails::Enable);
        config.wasm_component_model(true);
        config.async_support(true);
        let wasm_engine = Engine::new(&config).expect("could not create engine");

        let fs_root = format!("/home/{}/.raikiri", whoami::username());
        let username = whoami::username();

        let (event_sender, event_receiver) = tokio::sync::mpsc::channel(0xFFFF);
        let event_receiver = Arc::new(Mutex::new(event_receiver));

        Self {
            fs_root,
            username,
            wasm_engine,
            component_registry: new_empty_cache(),
            secrets_cache: new_empty_cache(),
            port: 0,
            conf_file: ConfFile::build().unwrap(),
            event_sender,
            event_receiver,
            event_handler: None,
            db_connections: scc::HashMap::default()
        }
    }

    pub async fn init(&mut self) -> Result<&mut Self, ThreadSafeError> {

        println!("Registering components...");
        self.component_registry = self.build_registry().await?;
        println!("Successfully registered components");

        let _self = self.clone();

        tokio::spawn(async move {
            while let Some(message) = _self.event_receiver.lock().await.recv().await {
                _self.event_handler.unwrap_or_else(|| default_event_handler)(message)
            }
        });

        Ok(self)
    }

    pub fn with_username(&mut self, username: String) -> Self {
        self.username = username;
        self.clone()
    }

    pub fn with_fs_root(&mut self, fs_root: String) -> Self {
        self.fs_root = fs_root;
        self.clone()
    }

    pub fn with_port(&mut self, port: u16) -> Self {
        self.port = port;
        self.clone()
    }

    pub fn with_event_handler(&mut self, handler: fn(ComponentEvent) -> ()) -> &mut Self {
        self.event_handler = Some(handler);
        self
    }

}

pub enum ComponentEvent {
    Execution {
        username_component_name: String,
        stdout: Option<MemoryOutputPipe>,
        start: DateTime<chrono::Utc>,
        duration: i64,
        status: u16
    }
}

pub type ThreadSafeError = Box<dyn std::error::Error + Send + Sync + 'static>;

pub fn default_event_handler(message: ComponentEvent) {
    match message {
        ComponentEvent::Execution { stdout, username_component_name, start, duration, status } => {
            if let Some(stdout) = stdout {
                println!("Stdout from {username_component_name}: {}", String::from_utf8(stdout.contents().to_vec()).unwrap());
            }
            let start_text = start.to_rfc3339();
            println!("Started {username_component_name} at {start_text} and finished in {duration}ms. Status code: {status}");
        }
    }
}