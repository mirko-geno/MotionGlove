# MAPE - MotionGlove
Un guante periférico inalámbrico capaz de controlar una computadora, reemplazando el uso del mouse mediante un Raspberry Pi Pico W y un MPU6050

#### Componentes
- Raspberry Pi Pico W
- MPU6050
- Bateria de litio
- Sensores de fuerza
- Teclado de membrana


## Para instalar el firmware en la RP Pico W:
### Requerimientos para flashear el micro
1. Se necesita instalar elf2uf2-rs, el target thumbv6m-none-eabi y en caso de usar Linux: arm-none-eabi-binutils. Además, se debe habilitar la comunicación usb con el Raspberry Pi Pico W usando reglas Udev en sistemas linux. Todos estos requisitos están automatizados corriendo el siguiente script:
```bash
./setup.sh
```
2. Asegurarse que el micro esté en modo bootloader

3. Compilar y ejecutar los binarios
```bash
cd firmware
cargo run --release
```

## Para ejecutar el graficador 3D:
```bash
cargo run --release --bin plotter
```
