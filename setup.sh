#!/usr/bin/env bash
set -e

echo "=== Configurando entorno para Raspberry Pi Pico W + Embassy ==="

OS="$(uname -s || echo Windows)"

# Instalar target thumbv6m-none-eabi y elf2uf2-rs (Linux y Windows)
echo -e "\nInstalando target thumbv6m-none-eabi..."
rustup target add thumbv6m-none-eabi

echo -e "\nInstalando elf2uf2-rs..."
cargo install elf2uf2-rs

if [[ "$OS" == "Linux" ]]; then
    # Instalar herramientas arm-none-eabi en Linux
    echo "Instalando arm-none-eabi-binutils..."
    if command -v apt >/dev/null; then
        sudo apt update
        sudo apt install -y binutils-arm-none-eabi
    elif command -v pacman >/dev/null; then
        sudo pacman -S --noconfirm arm-none-eabi-binutils
    elif command -v dnf >/dev/null; then
        sudo dnf install -y arm-none-eabi-binutils-cs
    else
        echo "No se reconoció el gestor de paquetes. Instala arm-none-eabi-binutils manualmente."
    fi

    # Setup udev rules para Linux
    echo -e "\n=== Configurando reglas udev para Raspberry Pi Pico W ==="
    RULES_FILE="/etc/udev/rules.d/99-pico.rules"

    echo '
# Raspberry Pi Pico BOOTSEL
SUBSYSTEM=="tty", ATTRS{idVendor}=="2e8a", ATTRS{idProduct}=="0003", MODE="0666"
# Raspberry Pi Pico Embassy firmware USB-serial
SUBSYSTEM=="tty", ATTRS{idVendor}=="c0de", ATTRS{idProduct}=="cafe", MODE="0666"
' | sudo tee $RULES_FILE

    sudo udevadm control --reload-rules
    sudo udevadm trigger

    echo "=== Reglas udev configuradas! ==="

else
    # Windows
    echo "Sistema Windows detectado: no se requieren reglas udev ni binutils."
fi

echo -e "\n=== Entorno listo! ==="