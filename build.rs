fn main() {
    dotenvy::dotenv().ok();

    let app_env = std::env::var("APP_ENV").unwrap_or_else(|_| "dev".to_string());
    let url_key = if app_env == "prod" { "SERVER_URL_PROD" } else { "SERVER_URL_DEV" };

    if let Ok(val) = std::env::var(url_key) {
        println!("cargo:rustc-env=SERVER_URL={}", val);
    }

    for var in ["WIFI_SSID", "WIFI_PASSWORD", "API_KEY"] {
        if let Ok(val) = std::env::var(var) {
            println!("cargo:rustc-env={}={}", var, val);
        }
    }

    embuild::espidf::sysenv::output();
}
