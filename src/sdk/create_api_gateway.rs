use crate::adapters::raikirifs::ThreadSafeError;

use super::shared::get_cloud_url;

pub async fn create_api_gateway(yml_bytes: Vec<u8>, version: i32) -> Result<(), ThreadSafeError> {

    let raikiri_cloud_url = get_cloud_url().await;

    reqwest::Client::new()
        .post(format!("{raikiri_cloud_url}/api_gateways/{version}"))
        .body(yml_bytes)
        .send()
        .await?;

    Ok(())
}