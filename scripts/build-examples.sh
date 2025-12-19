#!/bin/bash
set -e
set -x

# Include common build script stuff
source scripts/build-common.sh

# Build Pico embassy example ...
EXAMPLE=embassy

# ... for the RP2040
TARGET=$RP2040_TARGET
cargo build --manifest-path=$EXAMPLES_MANIFEST_PATH --bin $EXAMPLE --target $TARGET --features rp2040-embassy
cargo build --manifest-path=$EXAMPLES_MANIFEST_PATH --bin $EXAMPLE --target $TARGET --features rp2040-embassy-defmt
cargo build --manifest-path=$EXAMPLES_MANIFEST_PATH --bin $EXAMPLE --target $TARGET --features rp2040-embassy,alloc
cargo build --manifest-path=$EXAMPLES_MANIFEST_PATH --bin $EXAMPLE --target $TARGET --features rp2040-embassy-defmt,alloc

# ... for the RP2350
TARGET=$RP2350_TARGET
cargo build --manifest-path=$EXAMPLES_MANIFEST_PATH --bin $EXAMPLE --target $TARGET --features rp2350-embassy
cargo build --manifest-path=$EXAMPLES_MANIFEST_PATH --bin $EXAMPLE --target $TARGET --features rp2350-embassy-defmt
cargo build --manifest-path=$EXAMPLES_MANIFEST_PATH --bin $EXAMPLE --target $TARGET --features rp2350-embassy,alloc
cargo build --manifest-path=$EXAMPLES_MANIFEST_PATH --bin $EXAMPLE --target $TARGET --features rp2350-embassy-defmt,alloc

# Build STM32 embassy example ...
EXAMPLE=embassy
TARGET=$STM32_TARGET
cargo build --manifest-path=$EXAMPLES_MANIFEST_PATH --bin $EXAMPLE --target $TARGET --features stm32-embassy
cargo build --manifest-path=$EXAMPLES_MANIFEST_PATH --bin $EXAMPLE --target $TARGET --features stm32-embassy-defmt
cargo build --manifest-path=$EXAMPLES_MANIFEST_PATH --bin $EXAMPLE --target $TARGET --features stm32-embassy,alloc
cargo build --manifest-path=$EXAMPLES_MANIFEST_PATH --bin $EXAMPLE --target $TARGET --features stm32-embassy-defmt,alloc

# Build nRF embassy example ...
EXAMPLE=embassy
TARGET=$NRF_TARGET
cargo build --manifest-path=$EXAMPLES_MANIFEST_PATH --bin $EXAMPLE --target $TARGET --features nrf-embassy
cargo build --manifest-path=$EXAMPLES_MANIFEST_PATH --bin $EXAMPLE --target $TARGET --features nrf-embassy-defmt
cargo build --manifest-path=$EXAMPLES_MANIFEST_PATH --bin $EXAMPLE --target $TARGET --features nrf-embassy,alloc
cargo build --manifest-path=$EXAMPLES_MANIFEST_PATH --bin $EXAMPLE --target $TARGET --features nrf-embassy-defmt,alloc

# Note: ESP32 support removed from examples due to riscv-rt native library conflict
# with RP2040/RP2350. Users can build ESP32 examples using task-watchdog-esp32 crate directly.

# Build rp-sync example ...
EXAMPLE=rp-sync
# ... for the RP2040
TARGET=$RP2040_TARGET
cargo build --manifest-path=$EXAMPLES_MANIFEST_PATH --bin $EXAMPLE --target $TARGET --features rp2040-hal-defmt
TARGET=$RP2350_TARGET
# ... for the RP2350
cargo build --manifest-path=$EXAMPLES_MANIFEST_PATH --bin $EXAMPLE --target $TARGET --features rp2350-hal-defmt

# Build intro example ...
EXAMPLE=intro
# ... for the RP2040
TARGET=$RP2040_TARGET
cargo build --manifest-path=$EXAMPLES_MANIFEST_PATH --bin $EXAMPLE --target $TARGET --features rp2040-embassy
# ... for the RP2350
TARGET=$RP2350_TARGET
cargo build --manifest-path=$EXAMPLES_MANIFEST_PATH --bin $EXAMPLE --target $TARGET --features rp2350-embassy

# Build ESP32 examples
echo "Building task-watchdog-esp32 examples..."
cd crates/task-watchdog-esp32
# Note: ESP32 builds require ESP environment to be sourced first: . ~/export-esp.sh
cargo build --example embassy --target xtensa-esp32-espidf --features defmt
cd ../..

echo "All example builds completed successfully!"

