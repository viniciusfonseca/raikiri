use tokio::fs::{self, DirEntry};
use yaml_rust2::{Yaml, YamlEmitter, YamlLoader};

use super::raikirifs::{self, ThreadSafeError};

struct ByteBuf<'a>(&'a [u8]);

impl<'a> std::fmt::LowerHex for ByteBuf<'a> {
    fn fmt(&self, fmtr: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        for byte in self.0 {
            fmtr.write_fmt(format_args!("{:02x}", byte))?;
        }
        Ok(())
    }
}

pub async fn get_component_secrets(username_component_name: String) -> Result<Vec<(String, String)>, ThreadSafeError> {

    let username = username_component_name.split(".").next().ok_or("error getting username")?;
    let username_hash = format!("{:x}", ByteBuf(&openssl::sha::sha256(&username.as_bytes())));
    let username_component_name_hash = format!("{:x}", ByteBuf(&openssl::sha::sha256(&username_component_name.as_bytes())));
    let secrets_path = format!("secrets/{username_hash}/{username_component_name_hash}");

    match raikirifs::exists(secrets_path.clone()).await {
        Ok(exists) => {
            if !exists {
                return Ok(Vec::new());
            }
        }
        Err(err) => {
            if let Some(err) = err.downcast_ref::<std::io::Error>() {
                if err.kind() == std::io::ErrorKind::NotFound {
                    return Ok(Vec::new());
                }
            }
            return Err(err);
        }
    }

    let encrypted = raikirifs::read_file(secrets_path.clone()).await?;
    let key = get_crypto_key(username.into()).await?;

    let decrypted = openssl::symm::decrypt(openssl::symm::Cipher::aes_256_cbc(), &key, None, &encrypted)?;
    let decrypted = String::from_utf8(decrypted)?;

    let secrets = YamlLoader::load_from_str(&decrypted)?[0].clone();
    let mut result_secrets = Vec::new();
    for (key, value) in secrets.as_hash().ok_or("error getting secrets")?.iter() {
        result_secrets.push((key.as_str().ok_or("error getting key")?.to_string(), value.as_str().ok_or("error getting value")?.to_string()));
    }
    Ok(result_secrets)
}

pub async fn gen_new_crypto_key() -> Result<Vec<u8>, ThreadSafeError> {
    let mut key = [0; 32];
    openssl::rand::rand_bytes(&mut key)?;
    Ok(key.to_vec())
}

pub async fn get_crypto_key(username: String) -> Result<Vec<u8>, ThreadSafeError> {
    
    let hash = format!("{:x}", ByteBuf(&openssl::sha::sha256(&username.as_bytes())));
    let key_path = format!("keys/{hash}");
    match raikirifs::exists(key_path.clone()).await {
        Ok(exists) => {
            if !exists {
                let key = gen_new_crypto_key().await?;
                raikirifs::write_file(key_path, key.clone()).await?;
                Ok(key)
            }
            else {
                raikirifs::read_file(key_path.into()).await
            }
        }
        Err(err) => {
            if let Some(err) = err.downcast_ref::<std::io::Error>() {
                if err.kind() == std::io::ErrorKind::NotFound {
                    let key = gen_new_crypto_key().await?;
                    raikirifs::write_file(key_path.clone(), key.clone()).await?;
                    Ok(key)
                }
                else {
                    // TODO: remove panic
                    panic!("error getting crypto key: {err}")
                }
            }
            else {
                Err(err)
            }
        }
    }
}

#[allow(unused)]
pub async fn update_crypto_key(username: String, new_key: Vec<u8>) -> Result<(), ThreadSafeError> {

    let raikiri_home = raikirifs::get_raikiri_home()?;
    let username_hash: String = format!("{:x}", ByteBuf(&openssl::sha::sha256(&username.as_bytes())));
    fs::create_dir_all(format!("{raikiri_home}/secrets/{username_hash}")).await.expect("error creating secrets directory");
    let secrets_path = format!("{raikiri_home}/secrets/{username_hash}");
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

#[allow(unused)]
async fn update_encrypted_secret(entry: DirEntry, username_hash: &String, current_key: &Vec<u8>, new_key: &Vec<u8>) -> Result<(), ThreadSafeError> {
    let path = entry.path();
    let file_name = path.file_name().unwrap().to_str().unwrap();
    let encrypted = raikirifs::read_file(format!("secrets/{username_hash}/{file_name}").into()).await?;
    let decrypted = openssl::symm::decrypt(openssl::symm::Cipher::aes_256_cbc(), &current_key, None, &encrypted)?;
    let encrypted_new = openssl::symm::encrypt(openssl::symm::Cipher::aes_256_cbc(), &new_key, None, &decrypted)?;
    raikirifs::write_file(format!("secrets/{file_name}.new"), encrypted_new).await?;

    Ok(())
}

#[allow(unused)]
async fn remove_all_new_encrypted(username: &String) -> Result<(), ThreadSafeError> {

    let raikiri_home = raikirifs::get_raikiri_home()?;
    let username_hash: String = format!("{:x}", ByteBuf(&openssl::sha::sha256(&username.as_bytes())));
    let secrets_path = format!("{raikiri_home}/secrets/{username_hash}");
    let mut entries = fs::read_dir(secrets_path).await.expect("error reading secrets directory");
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        let file_name = path.file_name().unwrap().to_str().unwrap();
        if file_name.ends_with(".new") {
            fs::remove_file(path).await?;
        }
    }

    Ok(())
}

pub async fn serialize_yaml(yaml: Yaml) -> Result<String, tokio::task::JoinError> {
    tokio::spawn(async move {
        let mut output = String::new();
        let mut yaml_emitter = YamlEmitter::new(&mut output);
        yaml_emitter.dump(&yaml).expect("error dumping yaml");
        output
    }).await
}

pub async fn update_component_secrets(username_component_name: String, secrets_content: Vec<u8>) -> Result<(), ThreadSafeError> {

    let secrets = YamlLoader::load_from_str(&String::from_utf8(secrets_content)?)?;
    let secret = serialize_yaml(secrets[0].clone()).await?;

    let username = username_component_name.split(".").next().ok_or("error getting username")?;
    let username_hash = format!("{:x}", ByteBuf(&openssl::sha::sha256(&username.as_bytes())));
    let username_component_name_hash = format!("{:x}", ByteBuf(&openssl::sha::sha256(&username_component_name.as_bytes())));
    let raikiri_home = raikirifs::get_raikiri_home()?;
    let secrets_path = format!("{raikiri_home}/secrets/{username_hash}");
    fs::create_dir_all(&secrets_path).await?;

    let crypto_key = get_crypto_key(username.to_string()).await?;

    let encrypted = openssl::symm::encrypt(openssl::symm::Cipher::aes_256_cbc(), &crypto_key, None, &secret.as_bytes())?;
    raikirifs::write_file(format!("secrets/{username_hash}/{username_component_name_hash}"), encrypted).await?;

    Ok(())
}