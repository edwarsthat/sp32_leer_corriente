use embedded_svc::http::client::Client;
use embedded_svc::http::Method;
use embedded_svc::io::Write;
use esp_idf_svc::http::client::{Configuration, EspHttpConnection};

const URL: &str = env!("SERVER_URL");

pub fn enviar(estado: u32) {
    let config = Configuration {
        crt_bundle_attach: Some(esp_idf_svc::sys::esp_crt_bundle_attach),
        ..Default::default()
    };

    let connection = EspHttpConnection::new(&config).expect("Fallo al crear conexion HTTP");
    let mut client = Client::wrap(connection);

    let body = estado.to_string();
    let body_len = body.len().to_string();

    let headers = [
        ("Content-Type", "text/plain"),
        ("Content-Length", body_len.as_str()),
    ];

    let mut request = client
        .request(Method::Post, URL, &headers)
        .expect("Fallo al abrir request");

    request
        .write_all(body.as_bytes())
        .expect("Fallo al escribir body");

    let response = request.submit().expect("Fallo al enviar");
    log::info!("Respuesta: {}", response.status());
}
