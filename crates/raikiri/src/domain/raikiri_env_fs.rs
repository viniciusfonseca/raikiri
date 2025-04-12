use std::path::Path;

use async_trait::async_trait;

use crate::adapters::raikirifs::ThreadSafeError;

use super::raikiri_env::RaikiriEnvironment;

#[async_trait]
pub trait RaikiriEnvironmentFS {
    fn get_path(&self, path: impl AsRef<Path>) -> String;
    async fn setup_fs(&self) -> Result<(), ThreadSafeError>;
    async fn read_file(&self, path: impl AsRef<Path> + Send) -> Result<Vec<u8>, ThreadSafeError>;
    async fn write_file(&self, path: impl AsRef<Path> + Send, content: Vec<u8>) -> Result<(), ThreadSafeError>;
    async fn remove_file(&self, path: impl AsRef<Path> + Send) -> Result<(), ThreadSafeError>;
    async fn file_exists(&self, path: impl AsRef<Path> + Send) -> bool;
    async fn create_dir(&self, path: impl AsRef<Path> + Send) -> Result<(), ThreadSafeError>;
    async fn read_dir(&self, path: impl AsRef<Path> + Send) -> Result<Vec<String>, ThreadSafeError>;
}

#[async_trait]
impl RaikiriEnvironmentFS for RaikiriEnvironment {

    fn get_path(&self, path: impl AsRef<Path>) -> String {
        let fs_root = &self.fs_root;
        let mut result = String::new();
        result.push_str(fs_root);
        result.push('/');
        result.push_str(path.as_ref().to_str().unwrap());
        result
    }

    async fn setup_fs(&self) -> Result<(), ThreadSafeError> {

        self.create_dir("").await?;
        self.create_dir("components").await?;
        self.create_dir("secrets").await?;
        self.create_dir("keys").await?;

        Ok(())
    }
    async fn read_file(&self, path: impl AsRef<Path> + Send) -> Result<Vec<u8>, ThreadSafeError> {
        Ok(tokio::fs::read(self.get_path(path)).await?)
    }
    async fn write_file(&self, path: impl AsRef<Path> + Send, content: Vec<u8>) -> Result<(), ThreadSafeError> {
        Ok(tokio::fs::write(self.get_path(path), content).await?)
    }
    async fn remove_file(&self, path: impl AsRef<Path> + Send) -> Result<(), ThreadSafeError> {
        Ok(tokio::fs::remove_file(self.get_path(path)).await?)
    }
    async fn file_exists(&self, path: impl AsRef<Path> + Send) -> bool {
        tokio::fs::metadata(self.get_path(path)).await.is_ok()
    }
    async fn create_dir(&self, path: impl AsRef<Path> + Send) -> Result<(), ThreadSafeError> {
        Ok(tokio::fs::create_dir_all(self.get_path(path)).await?)
    }
    async fn read_dir(&self, path: impl AsRef<Path> + Send) -> Result<Vec<String>, ThreadSafeError> {
        let fs_root = &self.fs_root;
        let mut entries = tokio::fs::read_dir(self.get_path(path)).await?;
        let mut result = Vec::new();
        while let Some(entry) = entries.next_entry().await? {
            result.push(entry.file_name().into_string().unwrap());
        }
        Ok(result)
    }
}
