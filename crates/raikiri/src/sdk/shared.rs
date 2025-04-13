pub async fn get_cloud_url() -> String {
    match std::env::var("RAIKIRI_CLOUD_URL") {
        Ok(url) => url,
        // Err(_) => match raikirifs::read_file(".cloud-url".into()).await {
        //     Ok(url) => String::from_utf8(url).unwrap(),
        //     Err(_) => "https://raikiri.distanteagle.dev".to_string().to_string()
        // }
        Err(_) => "https://raikiri.distanteagle.dev".to_string()
    }
}