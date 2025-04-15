use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::adapters::db::postgresql::create_psql_connection;

use super::raikiri_env::{RaikiriEnvironment, ThreadSafeError};

pub enum RaikiriDBConnectionKind {
    POSTGRESQL,
    MYSQL,
    MONGODB,
    DYNAMODB
}

#[async_trait(?Send)]
pub trait RaikiriEnvironmentDB {
    async fn create_connection(&self, kind: RaikiriDBConnectionKind, params: Vec<u8>) -> Box<dyn RaikiriDBConnection + Send + Sync>; 
    async fn get_connection(&self, id: String) -> Arc<RwLock<Box<dyn RaikiriDBConnection + Send + Sync>>>;
}

#[async_trait(?Send)]
pub trait RaikiriDBConnection {
    async fn query(&self, params: Vec<u8>) -> Result<Vec<u8>, ThreadSafeError>;
    async fn execute_command(&self, params: Vec<u8>) -> Result<Vec<u8>, ThreadSafeError>;
}

#[async_trait(?Send)]
impl RaikiriEnvironmentDB for RaikiriEnvironment {
    async fn create_connection(&self, kind: RaikiriDBConnectionKind, params: Vec<u8>) -> Box<dyn RaikiriDBConnection + Send + Sync> {
        match kind {
            RaikiriDBConnectionKind::POSTGRESQL => Box::new(create_psql_connection(params).await.unwrap()),
            RaikiriDBConnectionKind::MYSQL => todo!(),
            RaikiriDBConnectionKind::MONGODB => todo!(),
            RaikiriDBConnectionKind::DYNAMODB => todo!(),
        }
    }
    async fn get_connection(&self, id: String) -> Arc<RwLock<Box<dyn RaikiriDBConnection + Send + Sync>>> {
        self.db_connections.get_entry_by_key(id, || panic!("Connection not found")).await
    }
}