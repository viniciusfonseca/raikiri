use homedir::get_my_home;
use openssl::encrypt;
use tokio::fs::{self, DirEntry};
use yaml_rust2::YamlLoader;

use super::raikirifs;

pub async fn get_component_secrets(username_component_name: String) -> Result<String, Box<dyn std::error::Error>> {

    let encrypted = raikirifs::read_file(format!("secrets/{username_component_name}.secret")).await?;
    let username = username_component_name.split(".").next().unwrap();
    let key = get_crypto_key(username.into()).await?;

    let decrypted = openssl::symm::decrypt(openssl::symm::Cipher::aes_256_cbc(), &key, None, &encrypted).unwrap();
    let decrypted = String::from_utf8(decrypted).unwrap();

    Ok(decrypted)
}

pub async fn get_crypto_key(username: String) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    
    raikirifs::read_file(format!("keys/{username}.key").into()).await
}

pub async fn update_crypto_key(username: String, new_key: Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {

    let raikiri_home = raikirifs::get_raikiri_home()?;
    let secrets_path = format!("{raikiri_home}/secrets/{username}");
    let mut entries = fs::read_dir(&secrets_path).await.expect("error reading secrets directory");
    let current_key = raikirifs::read_file(format!("keys/{username}.key").into()).await.expect("error reading current key");

    while let Some(entry) = entries.next_entry().await.unwrap() {
        if update_encrypted_secret(entry, &username, &current_key, &new_key).await.is_err() {
            remove_all_new_encrypted(&username).await;
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

async fn update_encrypted_secret(entry: DirEntry, username: &String, current_key: &Vec<u8>, new_key: &Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
    let path = entry.path();
    let file_name = path.file_name().unwrap().to_str().unwrap();
    let encrypted = raikirifs::read_file(format!("secrets/{username}/{file_name}").into()).await.unwrap();
    let decrypted = openssl::symm::decrypt(openssl::symm::Cipher::aes_256_cbc(), &current_key, None, &encrypted).unwrap();
    let encrypted_new = openssl::symm::encrypt(openssl::symm::Cipher::aes_256_cbc(), &new_key, None, &decrypted).unwrap();
    raikirifs::write_file(format!("secrets/{file_name}.new"), encrypted_new).await.unwrap();

    Ok(())
}

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

pub async fn update_component_secrets(username_component_name: String, secrets_content: Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {

    let secrets = YamlLoader::load_from_str(&String::from_utf8(secrets_content).unwrap()).expect("error parsing yaml");
    let secret = secrets[0].as_str().unwrap().to_string();

    let homedir = get_my_home()?.unwrap();
    let homedir = homedir.to_str().unwrap();
    let secret_path = format!("{homedir}/.raikiri/secrets/{username_component_name}.secret");
    fs::write(&secret_path, secret).await?;

    Ok(())
}