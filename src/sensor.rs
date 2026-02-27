use std::borrow::Borrow;

use esp_idf_svc::hal::adc::oneshot::{AdcChannelDriver, AdcDriver};
use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::hal::gpio::ADCPin;

// Umbral RMS: por encima = máquina encendida. Ajustar según logs.
pub const UMBRAL_CORRIENTE: u16 = 90;

// 4 ciclos a 60 Hz ≈ 67 ms. Muestreamos cada 1 ms → 67 muestras (~16 por ciclo).
// Calcula RMS-AC = sqrt(Σ((v - media)²) / N). Sin corriente → ~0.
pub fn leer_rms<'d, T, M>(pin: &mut AdcChannelDriver<'d, T, M>) -> u16
where
    T: ADCPin,
    M: Borrow<AdcDriver<'d, T::Adc>>,
{
    const N: u32 = 67;
    let mut muestras = [0u32; 67];

    // Primera pasada: recolectar muestras y calcular media
    let mut suma: u32 = 0;
    for m in muestras.iter_mut() {
        let v = pin.read().unwrap_or(0) as u32;
        *m = v;
        suma += v;
        FreeRtos::delay_ms(1);
    }
    let media = suma / N;

    // Segunda pasada: RMS de la componente AC (desviación respecto a la media)
    let mut suma_cuadrados: u32 = 0;
    for v in muestras.iter() {
        let ac = (*v as i32) - (media as i32);
        suma_cuadrados += (ac * ac) as u32;
    }
    ((suma_cuadrados / N) as f32).sqrt() as u16
}

pub fn hay_corriente<'d, T, M>(pin: &mut AdcChannelDriver<'d, T, M>) -> bool
where
    T: ADCPin,
    M: Borrow<AdcDriver<'d, T::Adc>>,
{
    leer_rms(pin) > UMBRAL_CORRIENTE
}
