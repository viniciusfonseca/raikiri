use async_trait::async_trait;
use serde::Deserialize;
use tokio_postgres::NoTls;

use crate::domain::{raikiri_env::ThreadSafeError, raikiri_env_db::RaikiriDBConnection};

#[derive(Deserialize)]
struct PostgreSQLExecuteParams {

}

#[derive(Deserialize)]
struct PostgreSQLQueryParams {

}

pub async fn create_psql_connection(params: Vec<u8>) -> tokio_postgres::Client {
    let connection_str = String::from_utf8(params).unwrap();
    let (client, connection) = tokio_postgres::connect(&connection_str, NoTls).await.unwrap();
    tokio::spawn(async move {
        connection.await.unwrap();
    });
    client
}

#[async_trait]
impl RaikiriDBConnection for tokio_postgres::Client {
    async fn execute(&self, params: Vec<u8>) -> Result<Vec<u8>, ThreadSafeError> {
        let params = serde_json::from_slice::<PostgreSQLExecuteParams>(&params).unwrap();
        Ok(Vec::new())       
    }

    async fn query(&self, params: Vec<u8>) -> Result<Vec<u8>, ThreadSafeError> {
        let params = serde_json::from_slice::<PostgreSQLQueryParams>(&params).unwrap();
        Ok(Vec::new())
    }
}