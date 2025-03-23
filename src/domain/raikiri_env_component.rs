use async_trait::async_trait;
use wasmtime::component::Component;

use crate::adapters::raikirifs::ThreadSafeError;

use super::{raikiri_env::RaikiriEnvironment, raikiri_env_fs::RaikiriEnvironmentFS};

#[async_trait]
trait RaikiriComponentStorage {
    async fn add_component(&self, user: String, name: String, path: String) -> Result<(), ThreadSafeError>;
    async fn get_component(&self, user: String, name: String) -> Result<Component, ThreadSafeError>;
    async fn remove_component(&self, user: String, name: String) -> Result<(), ThreadSafeError>;
}

#[async_trait]
impl RaikiriComponentStorage for RaikiriEnvironment {
    async fn add_component(&self, user: String, name: String, path: String) -> Result<(), ThreadSafeError> {
        let component_bytes = tokio::fs::read(path).await?;
        let component = Component::from_binary(&self.wasm_engine, &component_bytes).expect("error compiling wasm component");
        let component_bytes = component.serialize().expect("error serializing component to file");
        self.write_file(format!("components/{user}.{name}.aot.wasm"), component_bytes).await
    }

    async fn get_component(&self, user: String, name: String) -> Result<Component, ThreadSafeError> {
        let component_bytes = self.read_file(format!("components/{user}.{name}.aot.wasm")).await?;
        unsafe { Ok(Component::deserialize(&self.wasm_engine, &component_bytes).expect("error compiling wasm component")) }
    }

    async fn remove_component(&self, user: String, name: String) -> Result<(), ThreadSafeError> {
        self.remove_file(format!("components/{user}.{name}.aot.wasm")).await
    }
}