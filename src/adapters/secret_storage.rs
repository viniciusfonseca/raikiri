use tokio::fs::{self, DirEntry};
use yaml_rust2::{Yaml, YamlEmitter, YamlLoader};

use super::raikirifs::{self, ThreadSafeError};

pub async fn get_component_secrets(username_component_name: String) -> Result<Vec<(String, String)>, ThreadSafeError> {

    let username = username_component_name.split(".").next().unwrap();
    let username_hash = format!("{:x?}", openssl::sha::sha256(&username.as_bytes()));
    let hash = format!("{:x?}", openssl::sha::sha256(&username_component_name.as_bytes()));
    let secrets_path = format!("secrets/{username_hash}/{hash}");

    match raikirifs::exists(secrets_path.clone()).await {
        Ok(exists) => {
            if !exists {
                return Ok(Vec::new());
            }
        }
        Err(err) => {
            return Err(err);
        }
    }

    let encrypted = raikirifs::read_file(secrets_path.clone()).await?;
    let key = get_crypto_key(username.into()).await?;

    let decrypted = openssl::symm::decrypt(openssl::symm::Cipher::aes_256_cbc(), &key, None, &encrypted).unwrap();
    let decrypted = String::from_utf8(decrypted).unwrap();

    let secrets = YamlLoader::load_from_str(&decrypted).expect("error parsing yaml")[0].clone();
    let mut result_secrets = Vec::new();
    for (key, value) in secrets.as_hash().unwrap() {
        result_secrets.push((key.as_str().unwrap().to_string(), value.as_str().unwrap().to_string()));
    }
    Ok(result_secrets)
}

pub async fn gen_new_crypto_key() -> Result<Vec<u8>, ThreadSafeError> {
    let mut key = [0; 32];
    openssl::rand::rand_bytes(&mut key)?;
    Ok(key.to_vec())
}

pub async fn get_crypto_key(username: String) -> Result<Vec<u8>, ThreadSafeError> {
    
    let hash = format!("{:x?}", openssl::sha::sha256(&username.as_bytes()));
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
        Err(err) => Err(err)
    }
}

#[allow(unused)]
pub async fn update_crypto_key(username: String, new_key: Vec<u8>) -> Result<(), ThreadSafeError> {

    let raikiri_home = raikirifs::get_raikiri_home()?;
    let hash: String = format!("{:x?}", openssl::sha::sha256(&username.as_bytes()));
    let secrets_path = format!("{raikiri_home}/secrets/{hash}");
    let mut entries = fs::read_dir(&secrets_path).await.expect("error reading secrets directory");
    let current_key = raikirifs::read_file(format!("keys/{hash}.key").into()).await.expect("error reading current key");

    while let Some(entry) = entries.next_entry().await.unwrap() {
        if update_encrypted_secret(entry, &hash, &current_key, &new_key).await.is_err() {
            remove_all_new_encrypted(&hash).await;
            return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "error updating encrypted secrets")));
        }
    }

    let mut entries = fs::read_dir(&secrets_path).await.expect("error reading secrets directory");

    while let Some(entry) = entries.next_entry().await.unwrap() {
        let path = entry.path();
        let file_name = path.file_name().unwrap().to_str().unwrap();
        if file_name.ends_with(".new") { continue }
        let new_content = raikirifs::read_file(format!("secrets/{file_name}.new").into()).await.unwrap();
        raikirifs::write_file(format!("secrets/{file_name}").into(), new_content).await.unwrap();
        raikirifs::remove_file(format!("secrets/{file_name}.new").into()).await.unwrap();
    }
    remove_all_new_encrypted(&username).await;

    raikirifs::write_file(format!("keys/{username}.key"), new_key).await
}

#[allow(unused)]
async fn update_encrypted_secret(entry: DirEntry, username: &String, current_key: &Vec<u8>, new_key: &Vec<u8>) -> Result<(), ThreadSafeError> {
    let path = entry.path();
    let file_name = path.file_name().unwrap().to_str().unwrap();
    let encrypted = raikirifs::read_file(format!("secrets/{username}/{file_name}").into()).await.unwrap();
    let decrypted = openssl::symm::decrypt(openssl::symm::Cipher::aes_256_cbc(), &current_key, None, &encrypted).unwrap();
    let encrypted_new = openssl::symm::encrypt(openssl::symm::Cipher::aes_256_cbc(), &new_key, None, &decrypted).unwrap();
    raikirifs::write_file(format!("secrets/{file_name}.new"), encrypted_new).await.unwrap();

    Ok(())
}

#[allow(unused)]
async fn remove_all_new_encrypted(username: &String) {

    let raikiri_home = raikirifs::get_raikiri_home().unwrap();
    let secrets_path = format!("{raikiri_home}/secrets/{username}");
    let mut entries = fs::read_dir(secrets_path).await.expect("error reading secrets directory");
    while let Some(entry) = entries.next_entry().await.unwrap() {
        let path = entry.path();
        let file_name = path.file_name().unwrap().to_str().unwrap();
        if file_name.ends_with(".new") {
            fs::remove_file(path).await.unwrap();
        }
    }
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

    let secrets = YamlLoader::load_from_str(&String::from_utf8(secrets_content).unwrap()).expect("error parsing yaml");
    let secret = serialize_yaml(secrets[0].clone()).await?;

    let username = username_component_name.split(".").next().unwrap();
    let raikiri_home = raikirifs::get_raikiri_home()?;
    let secrets_path = format!("{raikiri_home}/secrets/{username}");
    fs::create_dir_all(&secrets_path).await?;

    let crypto_key = raikirifs::read_file(format!("keys/{username}.key").into()).await?;

    let encrypted = openssl::symm::encrypt(openssl::symm::Cipher::aes_256_cbc(), &crypto_key, None, &secret.as_bytes())?;
    raikirifs::write_file(format!("secrets/{username}/{username_component_name}.secret"), encrypted).await?;

    Ok(())
}