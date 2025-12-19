#!/bin/bash
set -e
set -x

# Include common build script stuff
source scripts/build-common.sh

# Build core library for all targets
echo "Building task-watchdog-core..."
cargo build -p task-watchdog-core --target $RP2040_TARGET
cargo build -p task-watchdog-core --target $RP2040_TARGET --features defmt
cargo build -p task-watchdog-core --target $RP2040_TARGET --features alloc
cargo build -p task-watchdog-core --target $RP2040_TARGET --features defmt,alloc

cargo build -p task-watchdog-core --target $RP2350_TARGET
cargo build -p task-watchdog-core --target $RP2350_TARGET --features defmt
cargo build -p task-watchdog-core --target $RP2350_TARGET --features alloc
cargo build -p task-watchdog-core --target $RP2350_TARGET --features defmt,alloc

cargo build -p task-watchdog-core --target $STM32_TARGET
cargo build -p task-watchdog-core --target $STM32_TARGET --features defmt
cargo build -p task-watchdog-core --target $STM32_TARGET --features alloc
cargo build -p task-watchdog-core --target $STM32_TARGET --features defmt,alloc

cargo build -p task-watchdog-core --target $NRF_TARGET
cargo build -p task-watchdog-core --target $NRF_TARGET --features defmt
cargo build -p task-watchdog-core --target $NRF_TARGET --features alloc
cargo build -p task-watchdog-core --target $NRF_TARGET --features defmt,alloc

cargo build -p task-watchdog-core --target $ESP32_TARGET
cargo build -p task-watchdog-core --target $ESP32_TARGET --features defmt
cargo build -p task-watchdog-core --target $ESP32_TARGET --features alloc
cargo build -p task-watchdog-core --target $ESP32_TARGET --features defmt,alloc

# Build RP2040/RP2350 platform crate
echo "Building task-watchdog-rp (RP2040)..."
cd crates/task-watchdog-rp
cargo build --target $RP2040_TARGET --no-default-features --features rp2040-embassy
cargo build --target $RP2040_TARGET --no-default-features --features rp2040-embassy,defmt
cargo build --target $RP2040_TARGET --no-default-features --features rp2040-embassy,alloc
cargo build --target $RP2040_TARGET --no-default-features --features rp2040-embassy,defmt,alloc
cargo build --target $RP2040_TARGET --no-default-features --features rp2040-hal
cargo build --target $RP2040_TARGET --no-default-features --features rp2040-hal,defmt
cargo build --target $RP2040_TARGET --no-default-features --features rp2040-hal,defmt,alloc

echo "Building task-watchdog-rp (RP2350)..."
cargo build --target $RP2350_TARGET --no-default-features --features rp2350-embassy
cargo build --target $RP2350_TARGET --no-default-features --features rp2350-embassy,defmt
cargo build --target $RP2350_TARGET --no-default-features --features rp2350-embassy,alloc
cargo build --target $RP2350_TARGET --no-default-features --features rp2350-embassy,defmt,alloc
cargo build --target $RP2350_TARGET --no-default-features --features rp2350-hal
cargo build --target $RP2350_TARGET --no-default-features --features rp2350-hal,defmt
cargo build --target $RP2350_TARGET --no-default-features --features rp2350-hal,defmt,alloc
cd ../..

# Build STM32 platform crate
echo "Building task-watchdog-stm32..."
cd crates/task-watchdog-stm32
TARGET=$STM32_TARGET
cargo build --target $TARGET
cargo build --target $TARGET --features defmt
cargo build --target $TARGET --features alloc
cargo build --target $TARGET --features defmt,alloc
cd ../..

# Build nRF platform crate
echo "Building task-watchdog-nrf..."
cd crates/task-watchdog-nrf
TARGET=$NRF_TARGET
cargo build --target $TARGET
cargo build --target $TARGET --features defmt
cargo build --target $TARGET --features alloc
cargo build --target $TARGET --features defmt,alloc
cd ../..

# Build ESP32 platform crate
echo "Building task-watchdog-esp32..."
cd crates/task-watchdog-esp32
TARGET=$ESP32_TARGET
cargo build --target $TARGET
cargo build --target $TARGET --features defmt
cargo build --target $TARGET --features alloc
cargo build --target $TARGET --features defmt,alloc
cd ../..

echo "All builds completed successfully!"