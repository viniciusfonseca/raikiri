use waki::{handler, ErrorCode, Request, Response};

#[handler]
fn hello(_req: Request) -> Result<Response, ErrorCode> {
    println!("Data from stdout");
    Response::builder()
        .body(format!(
            "Hello World!"
        ))
        .build()
}

fn main() {}