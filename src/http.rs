use embedded_svc::http::client::Client;
use embedded_svc::http::Method;
use embedded_svc::io::Write;
use esp_idf_svc::http::client::{Configuration, EspHttpConnection};

const URL: &str = env!("SERVER_URL");

pub fn enviar(estado: u32) {
    let config = Configuration {
        crt_bundle_attach: Some(esp_idf_svc::sys::esp_crt_bundle_attach),
        use_global_ca_store: true,
        timeout: Some(std::time::Duration::from_secs(30)),
        ..Default::default()
    };

    let connection = match EspHttpConnection::new(&config) {
        Ok(c) => c,
        Err(e) => {
            log::error!("Fallo al crear conexion HTTP: {:?}", e);
            return;
        }
    };

    let mut client = Client::wrap(connection);

    let body = estado.to_string();
    let body_len = body.len().to_string();

    let headers = [
        ("Content-Type", "text/plain"),
        ("Content-Length", body_len.as_str()),
    ];

    let mut request = match client.request(Method::Post, URL, &headers) {
        Ok(r) => r,
        Err(e) => {
            log::error!("Fallo al conectar con el servidor: {:?}", e);
            return;
        }
    };

    if let Err(e) = request.write_all(body.as_bytes()) {
        log::error!("Fallo al escribir body: {:?}", e);
        return;
    }

    match request.submit() {
        Ok(response) => log::info!("Respuesta: {}", response.status()),
        Err(e) => log::error!("Fallo al enviar: {:?}", e),
    }
}
