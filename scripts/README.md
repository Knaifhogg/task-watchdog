# Scripts

All scripts expect to be run in the root directory of the repository.

## Building

Build scripts are provided for the split-crate architecture:

### `build-lib.sh`
Builds the core library and all platform-specific implementations for all supported targets and feature combinations.

```bash
scripts/build-lib.sh
```

### `build-examples.sh`
Builds all example binaries for supported platforms (RP2040, RP2350, STM32, nRF52840). 

**Note:** ESP32 examples are not included due to native library conflicts (`riscv-rt` v0.12 vs v0.16).

```bash
scripts/build-examples.sh
```

### `build-all.sh`
Convenience script that runs both `build-lib.sh` and `build-examples.sh`.

```bash
scripts/build-all.sh
```

This requires the installation of the following targets:

```bash
rustup target add thumbv6m-none-eabi         # RP2040/Pico
rustup target add thumbv8m.main-none-eabihf  # RP2350/Pico 2
rustup target add thumbv7m-none-eabi         # STM32
rustup target add thumbv7em-none-eabi        # nRF52840
```

## Flashing Examples

Helper scripts are provided to build and flash examples to various platforms via debug probe:

- `flash-async-pico.sh` - Flash `embassy` example to RP2040
- `flash-async-pico2.sh` - Flash `embassy` example to RP2350
- `flash-async-stm32f103c8.sh` - Flash `embassy` example to STM32F103C8 (Blue Pill)
- `flash-async-nrf52840.sh` - Flash `embassy` example to nRF52840
- `flash-sync-pico.sh` - Flash synchronous `rp-sync` example to RP2040
- `flash-sync-pico2.sh` - Flash synchronous `rp-sync` example to RP2350

Example usage:

```bash
scripts/flash-async-pico.sh
```

## ESP32

### Examples

ESP32 examples are located in a separate directory to avoid native library conflicts. The main examples/Cargo.toml does not include ESP32 support due to a `riscv-rt` linking conflict: RP2040/RP2350 require `riscv-rt` v0.12, while ESP32 requires v0.16 via `esp-riscv-rt`.

Build and flash the ESP32 embassy example:

```bash
scripts/flash-async-esp32.sh
```

Or manually:

```bash
cd crates/task-watchdog-esp32
. ~/export-esp.sh
cargo run --example embassy --target xtensa-esp32-espidf --features defmt
```

### Setup

ESP32 support requires additional tools. See the [ESP on Rust Book](https://docs.esp-rs.org/book/) for complete setup instructions.

Quick setup:

```bash
cargo install espup
cargo install espflash
cargo install cargo-espflash
espup install
```

Then source the ESP environment before building:

```bash
. ~/export-esp.sh
```

