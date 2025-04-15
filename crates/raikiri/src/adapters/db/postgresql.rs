use core::panic;
use std::pin::pin;

use async_trait::async_trait;
use futures::StreamExt;
use serde::Deserialize;
use serde_json::{json, Value};
use tokio_postgres::{types::ToSql, GenericClient, NoTls};

use crate::domain::{raikiri_env::ThreadSafeError, raikiri_env_db::RaikiriDBConnection};

#[derive(Deserialize)]
struct PostgreSQLParams {
    sql: String,
    params: Vec<Value>
}

pub async fn create_psql_connection(params: Vec<u8>) -> Result<tokio_postgres::Client, ThreadSafeError> {
    let connection_str = String::from_utf8(params)?;
    let (client, connection) = tokio_postgres::connect(&connection_str, NoTls).await?;
    tokio::spawn(async move {
        connection.await.unwrap();
    }).await?;
    Ok(client)
}

fn cast_value_as_tosql(v: Value) -> Box<dyn ToSql + Sync> {
    match v {
        Value::Null => Box::new(Option::<String>::None),
        Value::Bool(v) => Box::new(v),
        Value::Number(v) => {
            if v.is_i64() {
                Box::new(v.as_i64().unwrap())
            }
            else if v.is_f64() {
                Box::new(v.as_f64().unwrap())
            }
            else {
                Box::new(v.as_i64().unwrap())
            }
        },
        Value::String(v) => Box::new(v),
        Value::Array(_) => panic!("Array value"),
        Value::Object(_) => panic!("Object value"),
    }
}

#[async_trait(?Send)]
impl RaikiriDBConnection for tokio_postgres::Client {
    
    async fn execute_command(&self, params: Vec<u8>) -> Result<Vec<u8>, ThreadSafeError> {
        let params = serde_json::from_slice::<PostgreSQLParams>(&params).unwrap();
        let stmt = self.prepare(&params.sql).await?;
        let params = params.params.iter()
            .map(|v| cast_value_as_tosql(v.clone()))
            .collect::<Vec<Box<dyn ToSql + Sync>>>();
        let params = params.iter()
            .map(|v| v.as_ref())
            .collect::<Vec<&(dyn ToSql + Sync)>>();
        let params = slice_iter(&params);
        let result = self.execute_raw(&stmt, params).await?;
        Ok(result.to_string().as_bytes().to_vec())
    }    

    async fn query(&self, params: Vec<u8>) -> Result<Vec<u8>, ThreadSafeError> {
        let params = serde_json::from_slice::<PostgreSQLParams>(&params).unwrap();
        let stmt = self.prepare(&params.sql).await?;
        let params = params.params.iter()
            .map(|v| cast_value_as_tosql(v.clone()))
            .collect::<Vec<Box<dyn ToSql + Sync>>>();
        let params = params.iter()
            .map(|v| v.as_ref())
            .collect::<Vec<&(dyn ToSql + Sync)>>();
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
    s: &'a [&'a (dyn ToSql + Sync)],
) -> impl ExactSizeIterator<Item = &'a dyn ToSql> + 'a {
    s.iter().map(|s| *s as _)
}

#[cfg(test)]
mod tests {
    use crate::domain::{raikiri_env_fs::RaikiriEnvironmentFS, tests::create_test_env};

    #[tokio::test]
    async fn test_postgresql() {
        let env = create_test_env();
        env.setup_fs().await.unwrap();

        
    }
}