use raikiri_wasi_sdk::*;

#[handler]
fn hello(_req: Request) -> Result<Response, ErrorCode> {

    let connection = SqlConnectionBuilder::new()
        .with_connection_type("postgres")
        .build();

    let _rows_affected = connection.execute_sql("INSERT INTO accounts (id, balance) VALUES ('1', 0);", &[] as &[&str]);

    let rows = connection.query_sql("SELECT id, balance FROM accounts", &[] as &[&str]);

    Response::builder()
        .body(rows)
        .build()
}

fn main() {}