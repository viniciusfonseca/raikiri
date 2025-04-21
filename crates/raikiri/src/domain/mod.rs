pub mod raikiri_env;
pub mod raikiri_env_fs;
pub mod raikiri_env_component;
pub mod raikiri_env_secrets;
pub mod raikiri_env_invoke;
pub mod raikiri_env_server;
pub mod raikiri_env_db;

#[cfg(test)]
pub mod tests {
    use http::Request;
    use http_body_util::combinators::BoxBody;
    use hyper::body::Bytes;

    use super::{raikiri_env::RaikiriEnvironment, raikiri_env_server::RaikiriEnvironmentServer};

    impl Drop for RaikiriEnvironment {
        fn drop(&mut self) {
            _ = std::fs::remove_dir_all(self.fs_root.clone());
        }
    }

    pub fn create_test_dir() -> String {
        let random_uuid = uuid::Uuid::new_v4().to_string();
        let tmp_path = format!("/tmp/raikiri-{random_uuid}");
        std::fs::create_dir_all(&tmp_path).unwrap();
        tmp_path
    }

    pub fn create_test_env() -> RaikiriEnvironment {
        RaikiriEnvironment::new()
            .with_username("test".to_string())
            .with_fs_root(create_test_dir())
    }

    pub async fn make_put_component_request(path: &str) -> Request<BoxBody<Bytes, hyper::Error>> {
        let component = tokio::fs::read(path).await.unwrap();
        let body: BoxBody<Bytes, hyper::Error> = RaikiriEnvironment::response_body_bytes(component).await;
        Request::builder()
            .uri("/")
            .method("POST")
            .header("Platform-Command", "Put-Component")
            .header("Component-Id", "hello")
            .body(body)
            .unwrap()
    }
}