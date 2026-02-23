use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::gpio::PinDriver;
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::nvs::EspDefaultNvsPartition;

mod http;
mod wifi;
mod sensor;

fn main() {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();
    log::info!("Iniciando aplicación");

    let peripherals = Peripherals::take().expect("Fallo al obtener perifericos");
    let corriente_driver = PinDriver::input(peripherals.pins.gpio34)
        .expect("Fallo al configurar pin de corriente");
    let sysloop = EspSystemEventLoop::take().expect("Fallo al obtener el sistema de eventos");
    let nvs = EspDefaultNvsPartition::take().expect("Fallo al obtener la particion NVS");

    let wifi = wifi::connect(peripherals.modem, sysloop, nvs);

    let ip_info = wifi
        .wifi()
        .sta_netif()
        .get_ip_info()
        .expect("Fallo al obtener informacion de IP");
    log::info!("Dirección IP asignada: {}", ip_info.ip);

    let mut estado_anterior = sensor::leer_corriente(&corriente_driver);
    log::info!("Estado inicial del sensor: {}", estado_anterior);
    http::enviar(estado_anterior as u32);
    log::info!("Estado inicial enviado al servidor: {}", estado_anterior as u32);

    loop {
        let corriente = sensor::leer_corriente(&corriente_driver);
        log::info!("Lectura actual: {}", corriente);
        if corriente != estado_anterior {
            log::info!("Estado cambió: {} -> {}", estado_anterior, corriente);
            estado_anterior = corriente;
            http::enviar(corriente as u32);
            log::info!("Dato enviado al servidor: {}", corriente as u32);
        }
        std::thread::sleep(std::time::Duration::from_secs(5));
    }
}
