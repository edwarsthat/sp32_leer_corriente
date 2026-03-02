use std::collections::VecDeque;
use std::sync::mpsc;
use std::thread;

use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::adc::attenuation::DB_11;
use esp_idf_svc::hal::adc::oneshot::config::AdcChannelConfig;
use esp_idf_svc::hal::adc::oneshot::{AdcChannelDriver, AdcDriver};
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::nvs::{EspDefaultNvsPartition, EspNvs};
use esp_idf_svc::sntp::{EspSntp, SyncStatus};

mod http;
mod sensor;
mod wifi;

fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn main() {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();
    log::info!("Iniciando aplicación");

    let peripherals = Peripherals::take().expect("Fallo al obtener perifericos");
    let sysloop = EspSystemEventLoop::take().expect("Fallo al obtener el sistema de eventos");
    let nvs = EspDefaultNvsPartition::take().expect("Fallo al obtener la particion NVS");

    // Contador de reinicios fallidos en NVS
    let nvs_boot = EspNvs::new(nvs.clone(), "boot_ctrl", true)
        .expect("Fallo al abrir namespace NVS");
    let reinicios: u8 = nvs_boot.get_u8("reinicios").ok().flatten().unwrap_or(0);

    if reinicios >= 3 {
        log::error!(
            "Modo seguro: {} reinicios fallidos consecutivos. Revisa el hardware del sensor.",
            reinicios
        );
        // Resetear contador: el modo seguro es una pausa, no un bloqueo permanente.
        // El próximo reset manual podrá intentar arrancar de nuevo.
        nvs_boot.set_u8("reinicios", 0).ok();
        loop {
            std::thread::sleep(std::time::Duration::from_secs(60));
        }
    }

    nvs_boot.set_u8("reinicios", reinicios + 1).ok();
    log::info!("Reinicio de arranque {}/3", reinicios + 1);

    let mut wifi = match wifi::connect_with_retry(peripherals.modem, sysloop, nvs.clone()) {
        Ok(w) => w,
        Err(e) => {
            log::error!("No se pudo establecer conexión WiFi tras 3 intentos: {:?}", e);
            log::error!("Entrando en modo de espera segura. Reinicia el dispositivo para reintentar.");
            loop {
                std::thread::sleep(std::time::Duration::from_secs(60));
            }
        }
    };

    let sntp = EspSntp::new_default().expect("Fallo al iniciar SNTP");
    log::info!("Esperando sincronizacion NTP...");
    let mut intentos_ntp = 0u8;
    while sntp.get_sync_status() != SyncStatus::Completed {
        std::thread::sleep(std::time::Duration::from_millis(500));
        intentos_ntp += 1;
        if intentos_ntp >= 60 {
            log::warn!("NTP no sincronizó en 30s, continuando con timestamp 0");
            break;
        }
    }
    if sntp.get_sync_status() == SyncStatus::Completed {
        log::info!("Hora sincronizada: {}", now_unix());
    }

    let ip_info = wifi
        .wifi()
        .sta_netif()
        .get_ip_info()
        .expect("Fallo al obtener informacion de IP");
    log::info!("Dirección IP asignada: {}", ip_info.ip);

    let (tx, rx) = mpsc::channel::<Result<(bool, u16), sensor::SensorError>>();

    // Hilo del sensor: lee ADC y envía por canal solo cuando cambia el estado
    let adc1 = peripherals.adc1;
    let gpio35 = peripherals.pins.gpio35;
    thread::Builder::new()
        .stack_size(12288)
        .spawn(move || {
            let adc_config = AdcChannelConfig { attenuation: DB_11, ..Default::default() };
            let adc = match AdcDriver::new(adc1) {
                Ok(a) => a,
                Err(e) => {
                    log::error!("Fallo al configurar ADC: {:?}", e);
                    tx.send(Err(sensor::SensorError::AdcInit)).ok();
                    return;
                }
            };

            let mut adc_pin = match AdcChannelDriver::new(&adc, gpio35, &adc_config) {
                Ok(p) => p,
                Err(e) => {
                    log::error!("Fallo al configurar pin ADC: {:?}", e);
                    tx.send(Err(sensor::SensorError::AdcPin)).ok();
                    return;
                }
            };

            // Suscribir este hilo al watchdog de hardware
            unsafe { esp_idf_svc::sys::esp_task_wdt_add(std::ptr::null_mut()) };

            let (corriente_inicial, rms_inicial) = match sensor::hay_corriente(&mut adc_pin) {
                Ok(c) => c,
                Err(e) => {
                    log::error!("Fallo al leer estado inicial del sensor: {:?}", e);
                    unsafe { esp_idf_svc::sys::esp_task_wdt_delete(std::ptr::null_mut()) };
                    tx.send(Err(e)).ok();
                    return;
                }
            };
            let mut estado = corriente_inicial;
            tx.send(Ok((corriente_inicial, rms_inicial))).ok(); // enviar estado inicial

            loop {
                let rms = match sensor::leer_rms(&mut adc_pin){
                    Ok(v) => v,
                    Err(e) => {
                        unsafe { esp_idf_svc::sys::esp_task_wdt_delete(std::ptr::null_mut()) };
                        tx.send(Err(e)).ok();
                        return;
                    }
                };
                // Alimentar el watchdog: prueba que el hilo no está colgado
                unsafe { esp_idf_svc::sys::esp_task_wdt_reset() };
                let corriente = rms > sensor::UMBRAL_CORRIENTE;
                log::info!(
                    "RMS AO: {}  |  Maquina: {}",
                    rms,
                    if corriente { "ENCENDIDA" } else { "APAGADA" }
                );

                if corriente != estado {
                    estado = corriente;
                    tx.send(Ok((corriente, rms))).ok();
                }
            }
        })
        .unwrap_or_else(|e| {
            log::error!("No se pudo crear hilo del sensor (sin memoria?): {:?}", e);
            unsafe { esp_idf_svc::sys::esp_restart() }
        });

    // Hilo principal: gestiona cola de envío independientemente del sensor
    let mut sensor_ok = false;
    let mut cola: VecDeque<(bool, u16, u64)> = VecDeque::new();
    loop {
        // Esperar nuevo evento del sensor hasta 5 segundos
        match rx.recv_timeout(std::time::Duration::from_secs(5)) {
            Ok(Ok((corriente, rms))) => {
                if !sensor_ok {
                    nvs_boot.set_u8("reinicios", 0).ok();
                    sensor_ok = true;
                    log::info!("Sensor OK, contador de reinicios reseteado");
                    log::info!(
                        "Estado inicial al arranque: {} (RMS={})",
                        if corriente { "ENCENDIDA" } else { "APAGADA" },
                        rms
                    );
                } else {
                    log::info!(
                        "Estado cambió: {} (RMS={})",
                        if corriente { "ENCENDIDA" } else { "APAGADA" },
                        rms
                    );
                }
                if cola.len() >= 50 {
                    cola.pop_front();
                    log::warn!("Cola llena, descartando evento mas antiguo");
                }
                cola.push_back((corriente, rms, now_unix()));
            }
            Ok(Err(e)) => {
                log::error!("Error en hilo sensor: {:?}", e);
                unsafe { esp_idf_svc::sys::esp_restart() };
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if !cola.is_empty() {
                    log::info!("Cola: {} evento(s) pendiente(s)", cola.len());
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                log::error!("Canal del sensor cerrado, reiniciando...");
                unsafe { esp_idf_svc::sys::esp_restart() };
            }
        }

        // Intentar vaciar la cola siempre, haya o no nuevo evento
        while let Some(&(estado, rms, ts)) = cola.front() {
            if !http::wifi_conectado(&wifi) {
                wifi::reconectar(&mut wifi);
                if !http::wifi_conectado(&wifi) {
                    log::warn!("Sin WiFi, {} eventos pendientes en cola", cola.len());
                    break;
                }
            }
            if http::enviar(estado, rms, ts) {
                cola.pop_front();
            } else {
                log::warn!("Fallo al enviar, {} eventos pendientes en cola", cola.len());
                break;
            }
        }
    }
}
