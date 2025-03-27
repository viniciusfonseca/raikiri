use std::sync::Arc;

use async_trait::async_trait;
use futures::{AsyncReadExt, AsyncWriteExt, StreamExt};
use vfs::async_vfs::AsyncFileSystem;

use crate::adapters::raikirifs::ThreadSafeError;

use super::raikiri_env::RaikiriEnvironment;

#[async_trait]
pub trait RaikiriEnvironmentFS {
    fn fs(&self) -> &Arc<dyn AsyncFileSystem>;
    async fn setup_fs(&self) -> Result<(), ThreadSafeError>;
    async fn read_file(&self, path: String) -> Result<Vec<u8>, ThreadSafeError>;
    async fn write_file(&self, path: String, content: Vec<u8>) -> Result<(), ThreadSafeError>;
    async fn remove_file(&self, path: String) -> Result<(), ThreadSafeError>;
    async fn file_exists(&self, name: String) -> bool;
    async fn create_dir(&self, name: String) -> Result<(), ThreadSafeError>;
    async fn read_dir(&self, path: String) -> Result<Vec<String>, ThreadSafeError>;
}

#[async_trait]
impl RaikiriEnvironmentFS for RaikiriEnvironment {

    fn fs(&self) -> &Arc<dyn AsyncFileSystem> { &self.fs }

    async fn setup_fs(&self) -> Result<(), ThreadSafeError> {
        let fs = &self.fs;
        let username = &self.username;

        fs.create_dir(&format!("/home/{username}/.raikiri")).await?;
        fs.create_dir(&format!("/home/{username}/.raikiri/components")).await?;
        fs.create_dir(&format!("/home/{username}/.raikiri/secrets")).await?;
        fs.create_dir(&format!("/home/{username}/.raikiri/keys")).await?;

        Ok(())
    }
    async fn read_file(&self, path: String) -> Result<Vec<u8>, ThreadSafeError> {
        let mut buf = Vec::new();
        let username = &self.username;
        &self.fs.open_file(&format!("/home/{username}/.raikiri/{path}")).await?.read_to_end(&mut buf).await?;
        Ok(buf)
    }
    async fn write_file(&self, path: String, content: Vec<u8>) -> Result<(), ThreadSafeError> {
        let username = &self.username;
        &self.fs.create_file(&format!("/home/{username}/.raikiri/{path}")).await?.write_all(&content).await?;
        Ok(())
    }
    async fn remove_file(&self, path: String) -> Result<(), ThreadSafeError> {
        let username = &self.username;
        &self.fs.remove_file(&format!("/home/{username}/.raikiri/{path}")).await?;
        Ok(())
    }
    async fn file_exists(&self, path: String) -> bool {
        let username = &self.username;
        self.fs.metadata(&format!("/home/{username}/.raikiri/{path}")).await.is_ok()
    }
    async fn create_dir(&self, path: String) -> Result<(), ThreadSafeError> {
        let username = &self.username;
        &self.fs.create_dir(&format!("/home/{username}/.raikiri/{path}")).await?;
        Ok(())
    }
    async fn read_dir(&self, path: String) -> Result<Vec<String>, ThreadSafeError> {
        let username = &self.username;
        let mut entries = self.fs.read_dir(&format!("/home/{username}/.raikiri/{path}")).await?;
        let mut result = Vec::new();
        while let Some(entry) = entries.next().await {
            result.push(entry);
        }
        Ok(result)
    }
}
