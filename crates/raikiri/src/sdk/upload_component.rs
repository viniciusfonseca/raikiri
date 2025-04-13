use crate::domain::raikiri_env::ThreadSafeError;

use super::shared::get_cloud_url;

pub async fn upload_component(username: String, component_name: String, file_path: String) -> Result<(), ThreadSafeError> {

    // let token = String::from_utf8(raikirifs::read_file(".cloud-token".to_string()).await?)?;
    let token = "YET TO IMPLEMENT!";

    let component_content = tokio::fs::read(file_path).await?;

    let multipart = reqwest::multipart::Part::bytes(component_content).file_name(format!("{username}.{component_name}.aot.wasm")).mime_str("application/wasm")?;
    let form = reqwest::multipart::Form::new()
        .part("component_name", reqwest::multipart::Part::text(component_name.clone()))
        .part("component_bytes", multipart);

    let raikiri_cloud_url = get_cloud_url().await;

    reqwest::Client::new()
        .post(format!("{raikiri_cloud_url}/components"))
        .header("Authorization", format!("Bearer {token}"))
        .multipart(form)
        .send()
        .await?;

    Ok(())
}