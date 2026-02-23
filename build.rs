fn main() {
    dotenvy::dotenv().ok();

    for var in ["WIFI_SSID", "WIFI_PASSWORD", "SERVER_URL"] {
        if let Ok(val) = std::env::var(var) {
            println!("cargo:rustc-env={}={}", var, val);
        }
    }

    embuild::espidf::sysenv::output();
}
