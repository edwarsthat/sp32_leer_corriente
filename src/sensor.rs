use std::borrow::Borrow;

use esp_idf_svc::hal::adc::oneshot::{AdcChannelDriver, AdcDriver};
use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::hal::gpio::ADCPin;

#[derive(Debug)]
pub enum SensorError {
    AdcInit,
    AdcPin,
    AdcRead,
}

// Umbral RMS: por encima = máquina encendida. Ajustar según logs.
pub const UMBRAL_CORRIENTE: u16 = 90;

// 4 ciclos a 60 Hz ≈ 67 ms. Muestreamos cada 1 ms → 67 muestras (~16 por ciclo).
// Calcula RMS-AC = sqrt(Σ((v - media)²) / N). Sin corriente → ~0.
pub fn leer_rms<'d, T, M>(pin: &mut AdcChannelDriver<'d, T, M>) -> Result<u16, SensorError>
where
    T: ADCPin,
    M: Borrow<AdcDriver<'d, T::Adc>>,
{
    const CICLOS: u32 = 4;
    const HZ: u32 = 60;
    const N: u32 = CICLOS * 1000 / HZ; // = 66, más explícito
    let mut muestras = [0u32; N as usize];

    // Primera pasada: recolectar muestras y calcular media
    let mut suma: u32 = 0;
    let mut errores: u8 = 0;
    let mut ultimo: u32 = 0;
    for m in muestras.iter_mut() {
        let v = match pin.read() {
            Ok(val) => {
                ultimo = val as u32;
                ultimo
            }
            Err(e) => {
                log::error!("Fallo al leer ADC: {:?}", e);
                errores += 1;
                if errores >= 5 {
                    return Err(SensorError::AdcRead);
                }
                ultimo // repite la última muestra válida
            }
        };
        *m = v;
        suma += v;
        FreeRtos::delay_ms(1);
    }
    let media = suma / N;

    // Segunda pasada: RMS de la componente AC (desviación respecto a la media)
    let mut suma_cuadrados: u64 = 0;
    for v in muestras.iter() {
        let ac = (*v as i32) - (media as i32);
        suma_cuadrados += (ac * ac) as u64;
    }
    Ok(((suma_cuadrados / N as u64) as f32).sqrt() as u16)
}

pub fn hay_corriente<'d, T, M>(
    pin: &mut AdcChannelDriver<'d, T, M>,
) -> Result<(bool, u16), SensorError>
where
    T: ADCPin,
    M: Borrow<AdcDriver<'d, T::Adc>>,
{
    let rms = leer_rms(pin)?;
    Ok((rms > UMBRAL_CORRIENTE, rms))
}
