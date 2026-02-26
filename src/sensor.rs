use std::borrow::Borrow;

use esp_idf_svc::hal::adc::oneshot::{AdcChannelDriver, AdcDriver};
use esp_idf_svc::hal::gpio::ADCPin;

// Umbral en mV: por encima = máquina encendida. Ajustar según logs.
pub const UMBRAL_CORRIENTE: u16 = 65;

// Toma 500 muestras en ~5 segundos (una cada 10ms) y retorna el máximo.
// Con AC a 60Hz el período es 16.7ms, así que muestreamos varias crestas.
pub fn leer_pico<'d, T, M>(pin: &mut AdcChannelDriver<'d, T, M>) -> u16
where
    T: ADCPin,
    M: Borrow<AdcDriver<'d, T::Adc>>,
{
    let mut max = 0u16;
    for _ in 0..500 {
        let v = pin.read().unwrap_or(0);
        if v > max {
            max = v;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    max
}

pub fn hay_corriente<'d, T, M>(pin: &mut AdcChannelDriver<'d, T, M>) -> bool
where
    T: ADCPin,
    M: Borrow<AdcDriver<'d, T::Adc>>,
{
    leer_pico(pin) > UMBRAL_CORRIENTE
}
