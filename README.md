# MAPE - MotionGlove
Un guante periférico inalámbrico capaz de controlar una computadora, reemplazando el uso del mouse mediante un Raspberry Pi Pico W y un MPU9250

#### Componentes
- 2 Raspberry Pi Pico W
- MPU9250
- Bateria de litio
- Sensores flex

## Para instalar el firmware en los RP Pico W:
### Requerimientos para flashear los micros:
1. Se necesita instalar elf2uf2-rs, el target thumbv6m-none-eabi y en caso de usar Linux: arm-none-eabi-binutils. Además, se debe habilitar la comunicación usb con el Raspberry Pi Pico W usando reglas Udev en sistemas linux. Todos estos requisitos están automatizados corriendo el siguiente script:
```bash
./setup.sh
```
2. Asegurarse que el micro esté en modo bootloader

3. Compilar y ejecutar los binarios acorde a cada dispositivo (Guante -> firmware, Dongle -> dongle_firmware):
```bash
cd firmware
cargo run --release
```
o
```bash
cd dongle_firmware
cargo run --release
```

## Para ejecutar el graficador 3D:
El graficador actualmente no está funcional, para hacerlo funcionar se debe modificar el código del firmware de los sensores (mpu) para que envíe cuaternios por el log serial respetando el formato esperado por el graficador. (Funciona en versiones viejas)
```bash 
cargo run --release --bin plotter
```
