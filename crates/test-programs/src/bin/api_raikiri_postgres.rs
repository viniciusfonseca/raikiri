use serde_json::json;
use waki::{handler, Client, ErrorCode, Request, Response};

#[handler]
fn hello(_req: Request) -> Result<Response, ErrorCode> {

    let connection_id = Client::new().post("https://raikiri.db/postgres_connection")
        .send().unwrap()
        .body().unwrap();

    let connection_id = String::from_utf8(connection_id).unwrap();

    let _rows_affected = Client::new().post("https://raikiri.db/execute")
        .header("Connection-Id", &connection_id)
        .body(json!({"sql": "INSERT INTO accounts (id, balance) VALUES ('1', 0)"}).to_string().as_bytes().to_vec())
        .send().unwrap()
        .body().unwrap();

    let rows = Client::new().post("https://raikiri.db/query")
        .header("Connection-Id", &connection_id)
        .body(json!({"sql": "SELECT id, balance FROM accounts"}).to_string().as_bytes().to_vec())
        .send().unwrap()
        .body().unwrap();

    Response::builder()
        .body(rows)
        .build()
}

fn main() {}