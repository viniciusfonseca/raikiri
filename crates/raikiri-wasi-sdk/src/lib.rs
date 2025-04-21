use serde::Serialize;
use serde_json::json;
use waki::Client;

pub use waki::{handler, ErrorCode, Request, Response};

trait DbConnection {
    fn execute(&self, params: Vec<u8>) -> i32;
    fn query(&self, params: Vec<u8>) -> Vec<u8>;
}

pub struct PostgresConnection {
    connection_id: String,
}

pub struct PgConnectionBuilder {
    connection_string_secret_name: Option<String>
}

impl PgConnectionBuilder {
    pub fn new() -> Self {
        Self {
            connection_string_secret_name: None
        }
    }

    pub fn from_secret(mut self, connection_string_secret_name: &str) -> Self {
        self.connection_string_secret_name = Some(connection_string_secret_name.to_string());
        self
    }

    pub fn build(self) -> PostgresConnection {

        let connection_id = Client::new().post("https://raikiri.db/postgres_connection")
            .header("Connection-String-Secret-Name", &self.connection_string_secret_name.unwrap_or("POSTGRES_CONNECTION_STRING".to_string()))
            .send().unwrap()
            .body().unwrap();

        PostgresConnection {
            connection_id: String::from_utf8(connection_id).unwrap(),
        }
    }
}

impl PostgresConnection {
    pub fn execute_sql(&self, sql: &str, params: &[impl Serialize]) -> i32 {
        let params = json!({"sql": sql, "params": params}).to_string();
        self.execute(params.as_bytes().to_vec())
    }
    pub fn query_sql(&self, sql: &str, params: &[impl Serialize]) -> Vec<u8> {
        let params = json!({"sql": sql, "params": params}).to_string();
        self.query(params.as_bytes().to_vec())
    }
}

impl DbConnection for PostgresConnection {
    fn execute(&self, params: Vec<u8>) -> i32 {
        let connection_id = Client::new().post("https://raikiri.db/execute")
            .header("Connection-Id", &self.connection_id)
            .body(params)
            .send().unwrap()
            .body().unwrap();

        // parse byte array as i32
        let connection_id = String::from_utf8(connection_id).unwrap();
        connection_id.parse().unwrap()
    }

    fn query(&self, params: Vec<u8>) -> Vec<u8> {
        Client::new().post("https://raikiri.db/query")
            .header("Connection-Id", &self.connection_id)
            .body(params)
            .send().unwrap()
            .body().unwrap()
    }
}