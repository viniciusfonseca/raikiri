pub mod raikiri_env;
pub mod raikiri_env_fs;
pub mod raikiri_env_component;
pub mod raikiri_env_secrets;
pub mod raikiri_env_invoke;
pub mod raikiri_env_server;
pub mod raikiri_env_db;

#[cfg(test)]
pub mod tests {
    use super::raikiri_env::RaikiriEnvironment;

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
}