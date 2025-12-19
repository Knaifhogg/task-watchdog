#!/bin/bash
set -e
set -x

# ESP32 example for task-watchdog
# Located in crates/task-watchdog-esp32/examples/ due to riscv-rt native library conflict
# with ARM targets (RP2040/RP2350)

# Install ESP tooling if not already installed
# cargo install espup espflash cargo-espflash

cd crates/task-watchdog-esp32

# Source ESP environment
. ~/export-esp.sh

# Build and flash the example
cargo run --example embassy --target xtensa-esp32-espidf --features defmt

cd ../..

