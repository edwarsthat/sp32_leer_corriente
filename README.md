# prueba1 — Monitor de corriente ESP32

Firmware en Rust para ESP32 que detecta presencia de corriente eléctrica mediante un sensor HW-671 y reporta los cambios de estado a un servidor HTTPS.

## ¿Qué hace?

1. Conecta al WiFi
2. Lee el estado del sensor HW-671 (GPIO 34) — `true` si hay corriente, `false` si no
3. Envía el estado inicial al servidor al arrancar
4. En el loop, solo envía un nuevo dato cuando el estado cambia

## Hardware

| Componente | Detalle |
|---|---|
| Microcontrolador | ESP32 |
| Sensor | HW-671 (detección de corriente AC/DC) |
| Pin de datos | GPIO 34 (salida digital DO del sensor) |
| Alimentación sensor | 5V (con nivel lógico compatible con ESP32) |

## Requisitos

- [Rust con toolchain `esp`](https://github.com/esp-rs/rust-build)
- `cargo-espflash`
- `espflash`

## Configuración

Crea un archivo `.env` en la raíz del proyecto:

```
WIFI_SSID="nombre_de_tu_red"
WIFI_PASSWORD="clave_del_wifi"
SERVER_URL="https://tu-servidor.com/endpoint/"
```

Las variables se incrustan en el binario en tiempo de compilación. El `.env` está en `.gitignore` y nunca se sube al repositorio.

## Compilar y flashear

```bash
# Solo compilar
cargo build --release

# Compilar y flashear (reemplaza el puerto según corresponda)
cargo espflash flash --release --port /dev/ttyUSB0 --monitor
```

> Antes de correr el comando, mantén presionado el botón `BOOT` del ESP32, luego presiona y suelta `RESET`, y suelta `BOOT`.

## Ver logs en tiempo real

```bash
espflash monitor --port /dev/ttyUSB0
```

## Estructura

```
src/
  main.rs     — inicialización y loop principal
  wifi.rs     — conexión WiFi
  http.rs     — envío de datos al servidor
  sensor.rs   — lectura del pin digital del sensor
```
