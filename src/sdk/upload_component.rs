use crate::adapters::raikirifs::{self, ThreadSafeError};

pub async fn upload_component(username: String, component_name: String, file_path: String) -> Result<(), ThreadSafeError> {

    let token = String::from_utf8(raikirifs::read_file(".cloud-token".to_string()).await?)?;

    let component_content = tokio::fs::read(file_path).await?;

    let multipart = reqwest::multipart::Part::bytes(component_content).file_name(format!("{username}.{component_name}.aot.wasm")).mime_str("application/wasm")?;
    let form = reqwest::multipart::Form::new()
        .part("component_name", reqwest::multipart::Part::text(component_name.clone()))
        .part("component_bytes", multipart);

    reqwest::Client::new()
        .post(format!("https://raikiri.distanteagle.dev/api/v1/components/{username}/{component_name}"))
        .header("Authorization", format!("Bearer {token}"))
        .multipart(form)
        .send()
        .await?;

    Ok(())
}