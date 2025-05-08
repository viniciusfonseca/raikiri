use serde::Serialize;
use serde_json::json;
use waki::Client;

pub use waki::{handler, ErrorCode, Request, Response};

trait DbConnection {
    fn execute(&self, params: Vec<u8>) -> i32;
    fn query(&self, params: Vec<u8>) -> Vec<u8>;
}

pub struct SqlConnection {
    connection_id: String,
}

pub struct SqlConnectionBuilder {
    connection_type: Option<String>,
    connection_string_secret_name: Option<String>
}

impl SqlConnectionBuilder {
    pub fn new() -> Self {
        Self {
            connection_type: None,
            connection_string_secret_name: None
        }
    }

    pub fn with_connection_type(mut self, connection_type: &str) -> Self {
        self.connection_type = Some(connection_type.to_string());
        self
    }

    pub fn with_connection_string_secret_name(mut self, connection_string_secret_name: &str) -> Self {
        self.connection_string_secret_name = Some(connection_string_secret_name.to_string());
        self
    }

    pub fn build(self) -> SqlConnection {

        let url = format!("https://raikiri.db/{}_connection", self.connection_type.unwrap());

        let connection_id = Client::new().post(&url)
            .header("Connection-String-Secret-Name", &self.connection_string_secret_name.unwrap_or("POSTGRES_CONNECTION_STRING".to_string()))
            .send().unwrap()
            .body().unwrap();

        SqlConnection {
            connection_id: String::from_utf8(connection_id).unwrap(),
        }
    }
}

impl SqlConnection {
    pub fn execute_sql(&self, sql: &str, params: &[impl Serialize]) -> i32 {
        let params = json!({"sql": sql, "params": params}).to_string();
        self.execute(params.as_bytes().to_vec())
    }
    pub fn query_sql(&self, sql: &str, params: &[impl Serialize]) -> Vec<u8> {
        let params = json!({"sql": sql, "params": params}).to_string();
        self.query(params.as_bytes().to_vec())
    }
}

impl DbConnection for SqlConnection {
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