
use esp_idf_svc::hal::gpio::{Input, InputPin, PinDriver};

pub fn leer_corriente<P: InputPin>(pin: &PinDriver<'_, P, Input>) -> bool {
    pin.is_high()
}
