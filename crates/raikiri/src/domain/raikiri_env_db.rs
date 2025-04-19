use std::sync::Arc;

use async_trait::async_trait;
use scc::hash_map::OccupiedEntry;
use tokio::sync::RwLock;

use crate::adapters::db::postgresql::create_psql_connection;

use super::raikiri_env::{RaikiriEnvironment, ThreadSafeError};

pub enum RaikiriDBConnectionKind {
    POSTGRESQL,
    MYSQL,
    MONGODB,
    DYNAMODB
}

#[async_trait]
pub trait RaikiriEnvironmentDB {
    async fn create_connection(&self, kind: RaikiriDBConnectionKind, params: Vec<u8>) -> Arc<dyn RaikiriDBConnection + Send + Sync>; 
    async fn get_connection(&self, id: String) -> OccupiedEntry<'_, std::string::String, Arc<dyn RaikiriDBConnection + Send + Sync + 'static>>;
}

#[async_trait]
pub trait RaikiriDBConnection {
    async fn query(&self, params: Vec<u8>) -> Result<Vec<u8>, ThreadSafeError>;
    async fn execute_command(&self, params: Vec<u8>) -> Result<Vec<u8>, ThreadSafeError>;
}

#[async_trait]
impl RaikiriEnvironmentDB for RaikiriEnvironment {
    async fn create_connection(&self, kind: RaikiriDBConnectionKind, params: Vec<u8>) -> Arc<dyn RaikiriDBConnection + Send + Sync> {
        match kind {
            RaikiriDBConnectionKind::POSTGRESQL => Arc::new(create_psql_connection(params).await.unwrap()),
            RaikiriDBConnectionKind::MYSQL => todo!(),
            RaikiriDBConnectionKind::MONGODB => todo!(),
            RaikiriDBConnectionKind::DYNAMODB => todo!(),
        }
    }
    async fn get_connection(&self, id: String) -> OccupiedEntry<'_, std::string::String, Arc<dyn RaikiriDBConnection + Send + Sync + 'static>> {
        self.db_connections.get_async(&id).await.unwrap()
    }
}