use homedir::get_my_home;

pub fn setup_app_dir() -> Result<(), Box<dyn std::error::Error>> {
    let home = get_my_home()?.unwrap();
    let home = home.to_str().unwrap();
    std::fs::create_dir_all(format!("{home}/.raikiri/modules"))?;
    Ok(())
}
