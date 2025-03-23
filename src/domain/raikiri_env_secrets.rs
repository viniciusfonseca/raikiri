use async_trait::async_trait;
use yaml_rust2::{Yaml, YamlEmitter, YamlLoader};

use crate::adapters::raikirifs::ThreadSafeError;

use super::{raikiri_env::RaikiriEnvironment, raikiri_env_fs::RaikiriEnvironmentFS};

#[async_trait]
pub trait RaikiriEnvironmentSecrets {
    async fn get_component_secrets_yaml(&self, user: String, name: String) -> Result<Yaml, ThreadSafeError>;
    async fn get_component_secrets(&self, user: String, name: String) -> Result<Vec<(String, String)>, ThreadSafeError>;
    async fn serialize_yaml(yaml: Yaml) -> Result<String, tokio::task::JoinError>;
    async fn get_crypto_key(&self, user: String) -> Result<Vec<u8>, ThreadSafeError>;
    fn gen_new_crypto_key() -> Result<Vec<u8>, ThreadSafeError>;
    async fn update_crypto_key(username: String, new_key: Vec<u8>) -> Result<(), ThreadSafeError>;
}

struct ByteBuf<'a>(&'a [u8]);
impl<'a> std::fmt::LowerHex for ByteBuf<'a> {
    fn fmt(&self, fmtr: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        for byte in self.0 {
            fmtr.write_fmt(format_args!("{:02x}", byte))?;
        }
        Ok(())
    }
}

#[async_trait]
impl RaikiriEnvironmentSecrets for RaikiriEnvironment {

    async fn get_component_secrets_yaml(&self, user: String, name: String) -> Result<Yaml, ThreadSafeError> {

        let username_hash = format!("{:x}", ByteBuf(&openssl::sha::sha256(&user.as_bytes())));
        let username_component_name_hash = format!("{:x}", ByteBuf(&openssl::sha::sha256(&format!("{user}.{name}").as_bytes())));
        let secrets_path = format!("secrets/{username_hash}/{username_component_name_hash}");

        if !self.file_exists(secrets_path.clone()).await { return Ok(Yaml::from_str("")) }

        let encrypted = self.read_file(secrets_path.clone()).await?;
        let key = &self.get_crypto_key(user).await?;

        let decrypted = openssl::symm::decrypt(openssl::symm::Cipher::aes_256_cbc(), &key, None, &encrypted)?;
        let decrypted = String::from_utf8(decrypted)?;

        Ok(YamlLoader::load_from_str(&decrypted)?[0].clone())

    }

    async fn get_component_secrets(&self, user: String, name: String) -> Result<Vec<(String, String)>, ThreadSafeError> {

        let secrets = self.get_component_secrets_yaml(user, name).await?;
        let mut result_secrets = Vec::new();
        for (key, value) in secrets.as_hash().ok_or("error getting secrets")?.iter() {
            result_secrets.push((
                key.as_str().ok_or("error getting key")?.to_string(),
                value.as_str().ok_or("error getting value")?.to_string()
            ));
        }
        Ok(result_secrets)
    }

    fn gen_new_crypto_key() -> Result<Vec<u8>, ThreadSafeError> {
        let mut key = [0; 32];
        openssl::rand::rand_bytes(&mut key)?;
        Ok(key.to_vec())
    }

    async fn get_crypto_key(&self, user: String) -> Result<Vec<u8>, ThreadSafeError> {

        let hash = format!("{:x}", ByteBuf(&openssl::sha::sha256(&user.as_bytes())));
        let key_path = format!("keys/{hash}");
        if self.file_exists(key_path.clone()).await {
            self.read_file(key_path.into()).await
        }
        else {
            let key = Self::gen_new_crypto_key()?;
            self.write_file(key_path, key.clone()).await?;
            Ok(key)
        }
    }

    async fn serialize_yaml(yaml: Yaml) -> Result<String, tokio::task::JoinError> {
        tokio::spawn(async move {
            let mut output = String::new();
            let mut yaml_emitter = YamlEmitter::new(&mut output);
            yaml_emitter.dump(&yaml).expect("error dumping yaml");
            output
        }).await
    }

    async fn update_crypto_key(&self, username: String, new_key: Vec<u8>) -> Result<(), ThreadSafeError> {

        let username_hash: String = format!("{:x}", ByteBuf(&openssl::sha::sha256(&username.as_bytes())));
        self.fs.create_dir(&format!("secrets/{username_hash}")).await.expect("error creating secrets directory");
        let mut entries = fs::read_dir(&secrets_path).await.expect("error reading secrets directory");
        let current_key = raikirifs::read_file(format!("keys/{username_hash}").into()).await.expect("error reading current key");
    
        while let Some(entry) = entries.next_entry().await? {
            if update_encrypted_secret(entry, &username_hash, &current_key, &new_key).await.is_err() {
                remove_all_new_encrypted(&username_hash).await;
                return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "error updating encrypted secrets")));
            }
        }
    
        let mut entries = fs::read_dir(&secrets_path).await?;
    
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            let file_name = path.file_name().unwrap().to_str().unwrap();
            if file_name.ends_with(".new") { continue }
            let new_content = raikirifs::read_file(format!("secrets/{username_hash}/{file_name}.new").into()).await?;
            raikirifs::write_file(format!("secrets/{username_hash}/{file_name}").into(), new_content).await?;
            raikirifs::remove_file(format!("secrets/{username_hash}/{file_name}.new").into()).await?;
        }
        remove_all_new_encrypted(&username_hash).await?;
    
        raikirifs::write_file(format!("keys/{username_hash}"), new_key).await
    }
}