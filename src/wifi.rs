use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::{modem::Modem, peripheral::Peripheral},
    nvs::EspDefaultNvsPartition,
    wifi::{BlockingWifi, ClientConfiguration, Configuration, EspWifi},
};

const SSID: &str = env!("WIFI_SSID");
const PASSWORD: &str = env!("WIFI_PASSWORD");

pub fn connect(
    modem: impl Peripheral<P = Modem> + 'static,
    sysloop: EspSystemEventLoop,
    nvs: EspDefaultNvsPartition,
) -> BlockingWifi<EspWifi<'static>> {
    let wifi = match EspWifi::new(modem, sysloop.clone(), Some(nvs)) {
        Ok(w) => w,
        Err(e) => {
            log::error!("Error al inicializar WiFi: {:?}", e);
            panic!("No se puede continuar sin WiFi");
        }
    };

    let mut wifi = match BlockingWifi::wrap(wifi, sysloop) {
        Ok(w) => w,
        Err(e) => {
            log::error!("Error al envolver WiFi en modo bloqueante: {:?}", e);
            panic!("No se puede continuar sin WiFi bloqueante");
        }
    };

    if let Err(e) = wifi.set_configuration(&Configuration::Client(ClientConfiguration {
        ssid: SSID.try_into().expect("SSID invalido o demasiado largo"),
        password: PASSWORD
            .try_into()
            .expect("Password invalida o demasiado larga"),
        ..Default::default()
    })) {
        log::error!("Error al configurar WiFi: {:?}", e);
        panic!("No se puede continuar sin configuracion WiFi");
    }

    wifi.start().expect("Fallo al iniciar WiFi");
    log::info!("WiFi iniciado, conectando...");

    wifi.connect().expect("Fallo al conectar WiFi");
    log::info!("Conectado a WiFi!");

    wifi.wait_netif_up()
        .expect("Fallo al esperar a que la interfaz de red esté activa");
    log::info!("Interfaz de red activa");

    wifi
}
