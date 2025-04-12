use homedir::get_my_home;

pub type ThreadSafeError = Box<dyn std::error::Error + Send + Sync>;

pub fn get_raikiri_home() -> Result<String, ThreadSafeError> {
    let home = get_my_home()?.unwrap();
    let home = home.to_str().unwrap();
    Ok(format!("{home}/.raikiri"))
}

pub async fn init() -> Result<(), ThreadSafeError> {
    let home = get_raikiri_home()?;
    std::fs::create_dir_all(format!("{home}/components"))?;
    std::fs::create_dir_all(format!("{home}/secrets"))?;
    std::fs::create_dir_all(format!("{home}/keys"))?;
    Ok(())
}

pub async fn read_file(path: String) -> Result<Vec<u8>, ThreadSafeError> {
    let home = get_raikiri_home()?;
    let file = tokio::fs::read(format!("{home}/{path}")).await?;
    Ok(file)
}

pub async fn write_file(path: String, content: Vec<u8>) -> Result<(), ThreadSafeError> {
    let home = get_raikiri_home()?;
    tokio::fs::write(format!("{home}/{path}"), content).await?;
    Ok(())
}

pub async fn remove_file(path: String) -> Result<(), ThreadSafeError> {
    let home = get_raikiri_home()?;
    tokio::fs::remove_file(format!("{home}/{path}")).await?;
    Ok(())
}

pub async fn exists(path: String) -> Result<bool, ThreadSafeError> {
    let home = get_raikiri_home()?;
    Ok(tokio::fs::metadata(format!("{home}/{path}")).await?.is_file())
}