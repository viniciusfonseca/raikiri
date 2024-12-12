use homedir::get_my_home;

pub fn get_raikiri_home() -> Result<String, Box<dyn std::error::Error>> {
    let home = get_my_home()?.unwrap();
    let home = home.to_str().unwrap();
    Ok(format!("{home}/.raikiri"))
}

pub async fn init() -> Result<(), Box<dyn std::error::Error>> {
    let home = get_raikiri_home()?;
    std::fs::create_dir_all(format!("{home}/components"))?;
    std::fs::create_dir_all(format!("{home}/secrets"))?;
    Ok(())
}

pub async fn read_file(path: String) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let home = get_raikiri_home()?;
    let file = tokio::fs::read(format!("{home}/{path}")).await?;
    Ok(file)
}

pub async fn write_file(path: String, content: Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
    let home = get_raikiri_home()?;
    tokio::fs::write(format!("{home}/{path}"), content).await?;
    Ok(())
}

pub async fn remove_file(path: String) -> Result<(), Box<dyn std::error::Error>> {
    let home = get_raikiri_home()?;
    tokio::fs::remove_file(format!("{home}/{path}")).await?;
    Ok(())
}