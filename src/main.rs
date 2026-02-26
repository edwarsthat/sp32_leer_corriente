use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::adc::attenuation::DB_11;
use esp_idf_svc::hal::adc::oneshot::config::AdcChannelConfig;
use esp_idf_svc::hal::adc::oneshot::{AdcChannelDriver, AdcDriver};
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::nvs::EspDefaultNvsPartition;

mod http;
mod sensor;
mod wifi;

fn main() {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();
    log::info!("Iniciando aplicación");

    let peripherals = Peripherals::take().expect("Fallo al obtener perifericos");

    // GPIO35 -> AO del sensor (salida analogica)
    let adc = AdcDriver::new(peripherals.adc1).expect("Fallo al configurar ADC");
    let adc_config = AdcChannelConfig { attenuation: DB_11, ..Default::default() };
    let mut adc_pin = AdcChannelDriver::new(&adc, peripherals.pins.gpio35, &adc_config)
        .expect("Fallo al configurar pin analogico");

    let sysloop = EspSystemEventLoop::take().expect("Fallo al obtener el sistema de eventos");
    let nvs = EspDefaultNvsPartition::take().expect("Fallo al obtener la particion NVS");

    let wifi = wifi::connect(peripherals.modem, sysloop, nvs);

    let ip_info = wifi
        .wifi()
        .sta_netif()
        .get_ip_info()
        .expect("Fallo al obtener informacion de IP");
    log::info!("Dirección IP asignada: {}", ip_info.ip);

    let mut estado_anterior = sensor::hay_corriente(&mut adc_pin);
    log::info!("Estado inicial del sensor: {}", estado_anterior);
    http::enviar(estado_anterior as u32);
    log::info!("Estado inicial enviado al servidor: {}", estado_anterior as u32);

    loop {
        let pico = sensor::leer_pico(&mut adc_pin);
        let corriente = pico > sensor::UMBRAL_CORRIENTE;
        log::info!("Pico AO: {} mV  |  Maquina: {}", pico, if corriente { "ENCENDIDA" } else { "APAGADA" });

        if corriente != estado_anterior {
            log::info!("Estado cambió: {} -> {}", estado_anterior, corriente);
            estado_anterior = corriente;
            http::enviar(corriente as u32);
            log::info!("Enviado al servidor: {}", corriente as u32);
        }
    }
}
