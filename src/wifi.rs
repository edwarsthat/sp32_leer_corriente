use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::{modem::Modem, peripheral::Peripheral},
    nvs::EspDefaultNvsPartition,
    wifi::{BlockingWifi, ClientConfiguration, Configuration, EspWifi},
};

#[derive(Debug)]
#[allow(dead_code)]
pub enum WifiError {
    InitFailed(esp_idf_svc::sys::EspError),
    ConfigFailed(esp_idf_svc::sys::EspError),
    ConnectionFailed(esp_idf_svc::sys::EspError),
    NetworkInterfaceTimeout,
}

const SSID: &str = env!("WIFI_SSID");
const PASSWORD: &str = env!("WIFI_PASSWORD");

pub fn connect_with_retry(
    modem: impl Peripheral<P = Modem> + 'static,
    sysloop: EspSystemEventLoop,
    nvs: EspDefaultNvsPartition,
) -> Result<BlockingWifi<EspWifi<'static>>, WifiError> {
    let esp_wifi =
        EspWifi::new(modem, sysloop.clone(), Some(nvs)).map_err(WifiError::InitFailed)?;

    let mut wifi = BlockingWifi::wrap(esp_wifi, sysloop).map_err(WifiError::InitFailed)?;

    wifi.set_configuration(&Configuration::Client(ClientConfiguration {
        ssid: SSID.try_into().map_err(|_| {
            log::error!("SSID inválido");
            WifiError::ConfigFailed(esp_idf_svc::sys::EspError::from(-1).unwrap())
        })?,
        password: PASSWORD.try_into().map_err(|_| {
            log::error!("Password inválido");
            WifiError::ConfigFailed(esp_idf_svc::sys::EspError::from(-1).unwrap())
        })?,
        ..Default::default()
    }))
    .map_err(WifiError::ConfigFailed)?;

    wifi.start().map_err(WifiError::ConnectionFailed)?;
    log::info!("WiFi iniciado, conectando...");

    let mut last_err = WifiError::NetworkInterfaceTimeout;
    for intento in 1u8..=3 {
        match wifi.connect() {
            Ok(_) => match wifi.wait_netif_up() {
                Ok(_) => {
                    log::info!("WiFi conectado en intento {}/3", intento);
                    return Ok(wifi);
                }
                Err(e) => {
                    log::warn!("Fallo al esperar netif en intento {}/3: {:?}", intento, e);
                    last_err = WifiError::NetworkInterfaceTimeout;
                }
            },
            Err(e) => {
                log::warn!("Fallo al conectar WiFi en intento {}/3: {:?}", intento, e);
                last_err = WifiError::ConnectionFailed(e);
            }
        }
        if intento < 3 {
            let _ = wifi.disconnect();
            std::thread::sleep(std::time::Duration::from_secs(5));
        }
    }

    Err(last_err)
}

pub fn reconectar(wifi: &mut BlockingWifi<EspWifi<'static>>) {
    log::warn!("Intentando reconectar WiFi...");
    let _ = wifi.disconnect();
    let _ = wifi.stop();
    match wifi.start() {
        Err(e) => {
            log::error!("Fallo al reiniciar driver WiFi: {:?}", e);
            return;
        }
        Ok(_) => {}
    }
    match wifi.connect() {
        Ok(_) => match wifi.wait_netif_up() {
            Ok(_) => log::info!("WiFi reconectado"),
            Err(e) => log::error!("Fallo al esperar red tras reconexion: {:?}", e),
        },
        Err(e) => log::error!("Fallo al reconectar WiFi: {:?}", e),
    }
}

