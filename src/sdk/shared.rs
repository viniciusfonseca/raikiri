pub fn get_cloud_url() -> String {
    std::env::var("RAIKIRI_CLOUD_URL").unwrap_or("raikiri.distanteagle.dev".into())
}