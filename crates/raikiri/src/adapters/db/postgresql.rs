use core::panic;
use std::pin::pin;

use async_trait::async_trait;
use futures::StreamExt;
use serde::Deserialize;
use serde_json::{json, Value};
use tokio_postgres::{types::ToSql, NoTls};

use crate::domain::{raikiri_env::ThreadSafeError, raikiri_env_db::RaikiriDBConnection};

#[derive(Deserialize)]
struct PostgreSQLParams {
    sql: String,
    params: Option<Vec<Value>>
}

pub async fn create_psql_connection(params: Vec<u8>) -> Result<tokio_postgres::Client, ThreadSafeError> {
    let connection_str = String::from_utf8(params)?;
    let (client, connection) = tokio_postgres::connect(&connection_str, NoTls).await?;
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });
    Ok(client)
}

fn cast_value_as_tosql(v: Value) -> Box<dyn ToSql + Sync + Send> {
    match v {
        Value::Null => Box::new(Option::<String>::None),
        Value::Bool(v) => Box::new(v),
        Value::Number(v) => {
            let vstr = v.to_string();
            if let Ok(vstr) = vstr.parse::<i32>() {
                return Box::new(vstr)
            }
            else if v.is_i64() {
                return Box::new(v.as_i64().unwrap())
            }
            else if v.is_f64() {
                return Box::new(v.as_f64().unwrap())
            }
            panic!("unsupported type")
        },
        Value::String(v) => Box::new(v),
        Value::Array(_) => panic!("Array value"),
        Value::Object(_) => panic!("Object value"),
    }
}

#[async_trait]
impl RaikiriDBConnection for tokio_postgres::Client {
    
    async fn execute_command(&self, params: Vec<u8>) -> Result<Vec<u8>, ThreadSafeError> {
        let params = serde_json::from_slice::<PostgreSQLParams>(&params).unwrap();
        let stmt = self.prepare(&params.sql).await?;
        let params = params.params.unwrap_or_default().iter()
            .map(|v| cast_value_as_tosql(v.clone()))
            .collect::<Vec<Box<dyn ToSql + Sync + Send>>>();
        let params = params.iter()
            .map(|v| v.as_ref())
            .collect::<Vec<&(dyn ToSql + Sync + Send)>>();
        let params = slice_iter(&params);
        let result = self.execute_raw(&stmt, params).await?;
        Ok(result.to_string().as_bytes().to_vec())
    }    

    async fn fetch_rows(&self, params: Vec<u8>) -> Result<Vec<u8>, ThreadSafeError> {
        let params = serde_json::from_slice::<PostgreSQLParams>(&params).unwrap();
        let stmt = self.prepare(&params.sql).await?;
        let params = params.params.unwrap_or_default().iter()
            .map(|v| cast_value_as_tosql(v.clone()))
            .collect::<Vec<Box<dyn ToSql + Sync + Send>>>();
        let params = params.iter()
            .map(|v| v.as_ref())
            .collect::<Vec<&(dyn ToSql + Sync + Send)>>();
        let params = slice_iter(&params);
        let mut rows = pin!(self.query_raw(&stmt, params).await?);
        let mut result = Vec::new();
        while let Some(Ok(row)) = rows.next().await {
            let mut map = serde_json::Map::new();
            for i in 0..stmt.columns().len() {
                let column = &stmt.columns()[i];
                let column_name = column.name().to_string();
                match &*column.type_() {
                    &tokio_postgres::types::Type::INT4 => {
                        let value: i32 = row.get(i);
                        map.insert(column_name, json!(value));
                    }
                    &tokio_postgres::types::Type::FLOAT8 => {
                        let value: f64 = row.get(i);
                        map.insert(column_name, json!(value));
                    }
                    &tokio_postgres::types::Type::TEXT => {
                        let value: String = row.get(i);
                        map.insert(column_name, json!(value));
                    }
                    &tokio_postgres::types::Type::NUMERIC => {
                        let value: i32 = row.get(i);
                        map.insert(column_name, json!(value));
                    }
                    &tokio_postgres::types::Type::BOOL => {
                        let value: bool = row.get(i);
                        map.insert(column_name, json!(value));
                    }
                    _ => {
                        let value: String = row.get(i);
                        map.insert(column_name, json!(value));
                    }
                }
            }
            result.push(map);
        }
        Ok(serde_json::to_string(&result).unwrap().as_bytes().to_vec())
    }
}

fn slice_iter<'a>(
    s: &'a [&'a (dyn ToSql + Sync + Send)],
) -> impl ExactSizeIterator<Item = &'a dyn ToSql> + 'a {
    s.iter().map(|s| *s as _)
}

#[cfg(test)]
mod tests {
    use crate::{adapters::db::postgresql::create_psql_connection, domain::{raikiri_env::ThreadSafeError, raikiri_env_db::RaikiriDBConnection, raikiri_env_fs::RaikiriEnvironmentFS, tests::create_test_env}};
    use serde_json::json;
    use testcontainers::runners::AsyncRunner;
    use testcontainers_modules::postgres;

    const POSTGRES_PORT: u16 = 5432;

    #[tokio::test]
    async fn test_postgresql_connection() -> Result<(), ThreadSafeError> {
        let env = create_test_env();
        env.setup_fs().await.unwrap();

        let pg_container = postgres::Postgres::default().start().await?;
        let host_port = pg_container.get_host_port_ipv4(POSTGRES_PORT).await?;
        let connection_string = &format!(
            "postgres://postgres:postgres@127.0.0.1:{host_port}/postgres",
        );

        let connection = create_psql_connection(connection_string.as_bytes().to_vec()).await?;

        let params = json!({"sql": "SELECT 1", "params": []}).to_string();
        let res = match connection.execute_command(params.into_bytes()).await {
            Ok(res) => res,
            Err(e) => {
                panic!("Failed to execute command: {e}")
            },
        };
        let res = String::from_utf8(res)?.parse::<i32>().unwrap();
        assert!(res > 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_fetch_rows() -> Result<(), ThreadSafeError> {

        let env = create_test_env();
        env.setup_fs().await?;

        let pg_container = postgres::Postgres::default().start().await?;
        let host_port = pg_container.get_host_port_ipv4(POSTGRES_PORT).await?;
        let connection_string = &format!(
            "postgres://postgres:postgres@127.0.0.1:{host_port}/postgres",
        );

        let connection = create_psql_connection(connection_string.as_bytes().to_vec()).await?;

        let params = json!({"sql": "CREATE TABLE accounts(id VARCHAR(255), balance INT);", "params": []}).to_string();
        connection.execute_command(params.into_bytes()).await?;

        let account_id = uuid::Uuid::new_v4().to_string();
        let params = json!({"sql": "INSERT INTO accounts (id, balance) VALUES ($1, $2);", "params": [account_id, 0]}).to_string();
        let res = connection.execute_command(params.into_bytes()).await?;
        let res = String::from_utf8(res)?.parse::<i32>().unwrap();
        assert!(res > 0);

        let params = json!({"sql": "SELECT id, balance FROM accounts", "params": []}).to_string();
        let res = connection.fetch_rows(params.into_bytes()).await?;
        let res = serde_json::from_slice::<Vec<serde_json::Value>>(&res)?;
        assert!(res[0].get("id").unwrap().as_str().unwrap() == account_id);
        assert!(res[0].get("balance").unwrap().as_i64().unwrap() == 0);

        Ok(())
    }
}