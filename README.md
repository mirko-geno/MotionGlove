# MAPE - MotionGlove
Un guante periférico inalámbrico capaz de controlar una computadora, reemplazando el uso del mouse mediante un Raspberry Pi Pico W y un MPU6050

#### Componentes
- Raspberry Pi Pico W
- MPU6050
- Bateria de litio
- Sensores de fuerza
- Teclado de membrana


## Para instalar el firmware en la RP Pico W:
Se puede hacer de dos maneras:
1. Desde el root: cargo build --release --bin firmware --target thumbv6m-none-eabi
2. Desde /firmware: cargo build --release

## Para ejecutar el graficador 3D:
Desde el root: cargo build --release --bin plotter