use embedded_svc::http::client::Client;
use embedded_svc::http::Method;
use embedded_svc::io::Write;
use esp_idf_svc::http::client::{Configuration, EspHttpConnection};
use esp_idf_svc::wifi::{BlockingWifi, EspWifi};

const URL: &str = env!("SERVER_URL");
const API_KEY: &str = env!("API_KEY");

pub fn wifi_conectado(wifi: &BlockingWifi<EspWifi<'static>>) -> bool {
    wifi.is_connected().unwrap_or(false)
}

pub fn enviar(estado: bool, rms: u16, ts: u64) -> bool {
    let config = Configuration {
        crt_bundle_attach: Some(esp_idf_svc::sys::esp_crt_bundle_attach),
        timeout: Some(std::time::Duration::from_secs(20)),
        ..Default::default()
    };

    let connection = match EspHttpConnection::new(&config) {
        Ok(c) => c,
        Err(e) => {
            log::error!("Fallo al crear conexion HTTP: {:?}", e);
            return false;
        }
    };

    let mut client = Client::wrap(connection);

    let body = format!("{{\"estado\":{},\"rms\":{},\"ts\":{}}}", estado as u8, rms, ts);
    let body_len = body.len().to_string();

    let headers = [
        ("Content-Type", "application/json"),
        ("Content-Length", body_len.as_str()),
        ("Authorization", API_KEY),
    ];

    let mut request = match client.request(Method::Post, URL, &headers) {
        Ok(r) => r,
        Err(e) => {
            log::error!("Fallo al conectar con el servidor: {:?}", e);
            return false;
        }
    };

    if let Err(e) = request.write_all(body.as_bytes()) {
        log::error!("Fallo al escribir body: {:?}", e);
        return false;
    }

    match request.submit() {
        Ok(response) => {
            let status = response.status();
            if status >= 200 && status < 300 {
                log::info!("Servidor OK: {}", status);
                true
            } else {
                log::error!("Servidor respondio con error HTTP: {}", status);
                false
            }
        }
        Err(e) => {
            log::error!("Fallo de red al enviar (DNS/TCP/TLS/timeout): {:?}", e);
            false
        }
    }
}
