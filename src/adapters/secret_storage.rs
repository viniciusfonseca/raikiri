use homedir::get_my_home;
use openssl::encrypt;
use tokio::fs;
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

pub async fn update_crypto_key(username: String, key_bytes: Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {

    let raikiri_home = raikirifs::get_raikiri_home()?;
    let secrets_path = format!("{raikiri_home}/secrets");
    let mut entries = fs::read_dir(secrets_path).await.expect("error reading secrets directory");
    let current_key = raikirifs::read_file(format!("keys/{username}.key").into()).await.expect("error reading current key");

    while let Some(entry) = entries.next_entry().await.unwrap() {
        
        let path = entry.path();
        let file_name = path.file_name().unwrap().to_str().unwrap();
        if !file_name.starts_with(&username) { continue }
        let encrypted = raikirifs::read_file(format!("secrets/{file_name}").into()).await.unwrap();
        let decrypted = openssl::symm::decrypt(openssl::symm::Cipher::aes_256_cbc(), &current_key, None, &encrypted).unwrap();
        let decrypted = String::from_utf8(decrypted).unwrap();
        let encrypted = openssl::symm::encrypt(openssl::symm::Cipher::aes_256_cbc(), &key_bytes, None, decrypted.as_bytes()).unwrap();
        raikirifs::write_file(format!("secrets/{file_name}"), encrypted).await.unwrap();
    }

    raikirifs::write_file(format!("keys/{username}.key"), key_bytes).await
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