use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;

use super::raikiri_env::{RaikiriEnvironment, ThreadSafeError};

pub enum RaikiriDBConnectionKind {
    PostgreSQL,
    MySQL,
    MongoDB,
    DynamoDB
}

#[async_trait]
pub trait RaikiriEnvironmentDB {
    async fn create_connection(&self, kind: RaikiriDBConnectionKind, params: Vec<u8>) -> Box<dyn RaikiriDBConnection + Send + Sync>; 
    async fn get_connection(&self, id: String) -> Arc<RwLock<Box<dyn RaikiriDBConnection + Send + Sync>>>;
}

#[async_trait]
pub trait RaikiriDBConnection {
    async fn execute_command(&mut self, params: Vec<u8>) -> Result<Vec<u8>, ThreadSafeError>;
}

#[async_trait]
impl RaikiriEnvironmentDB for RaikiriEnvironment {
    async fn create_connection(&self, kind: RaikiriDBConnectionKind, params: Vec<u8>) -> Box<dyn RaikiriDBConnection + Send + Sync> {
        match kind {
            RaikiriDBConnectionKind::PostgreSQL => todo!(),
            RaikiriDBConnectionKind::MySQL => todo!(),
            RaikiriDBConnectionKind::MongoDB => todo!(),
            RaikiriDBConnectionKind::DynamoDB => todo!(),
            _ => panic!("Unsupported database kind")
        }
    }
    async fn get_connection(&self, id: String) -> Arc<RwLock<Box<dyn RaikiriDBConnection + Send + Sync>>> {
        self.db_connections.get_entry_by_key(id, || panic!("Connection not found")).await
    }
}