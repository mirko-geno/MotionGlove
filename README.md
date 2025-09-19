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
1. Se necesita elf2uf2-rs, y se puede instalar con cargo usando:
```bash
cargo install elf2uf2-rs
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
