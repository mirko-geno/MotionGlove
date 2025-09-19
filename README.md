# MAPE - MotionGlove
Un guante periférico inalámbrico capaz de controlar una computadora, reemplazando el uso del mouse mediante un Raspberry Pi Pico W y un MPU6050

#### Componentes
- Raspberry Pi Pico W
- MPU6050
- Bateria de litio
- Sensores de fuerza
- Teclado de membrana


## Para instalar el firmware en la RP Pico W:
Asegurarse que el micro esté en modo bootloader

```bash
cd firmware
cargo run --release
```


## Para ejecutar el graficador 3D:
```bash
cargo run --release --bin plotter
```
