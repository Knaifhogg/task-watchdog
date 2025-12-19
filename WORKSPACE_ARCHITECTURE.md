# task-watchdog Workspace Architecture

## Problem Solved

The original monolithic crate had a dependency conflict preventing simultaneous support for all platforms:

- **RP2350** (via `rp235x-hal v0.4`) requires `riscv-rt ^0.12`
- **ESP32** (via `esp-hal v1.0`) requires `riscv-rt ^0.16` (via `esp-riscv-rt`)

The `riscv-rt` crate uses the `links = "riscv-rt"` attribute, which prevents multiple versions from being linked in the same binary. Cargo's workspace resolver includes ALL members' dependencies in a single resolution graph, making this conflict impossible to bypass with feature flags alone.

## Solution: Platform-Isolated Crates

The monolithic crate has been split into **6 separate crates**, each deployable independently:

```
crates/
├── task-watchdog-core/           # Platform-agnostic traits & generic impl
├── task-watchdog-rp/             # RP2040/RP2350 (Embassy + HAL sync)
├── task-watchdog-stm32/          # STM32 (Embassy async)
├── task-watchdog-nrf/            # nRF52840 (Embassy async)
└── task-watchdog-esp32/          # ESP32 (Embassy async)
```

### Workspace Configuration

**Root Cargo.toml** includes **ONLY** core as workspace member:

```toml
[workspace]
members = [
    "crates/task-watchdog-core",
]
exclude = [
    "crates/task-watchdog-rp",
    "crates/task-watchdog-stm32",
    "crates/task-watchdog-nrf",
    "crates/task-watchdog-esp32",
]
resolver = "2"
```

**Platform crates** (RP, STM32, nRF, ESP32) exist as **standalone crates** in the `crates/` directory but are **NOT workspace members**. This prevents their conflicting dependencies from being resolved together.

### Build Strategy

✅ **Isolated Platform Builds** (RECOMMENDED):
```bash
# Build RP2350 only - no ESP32 deps pulled
cd crates/task-watchdog-rp
cargo build --target thumbv8m.main-none-eabihf --features rp2350-embassy

# Build ESP32 only - no RP2350 deps pulled
cd crates/task-watchdog-esp32
cargo build --target xtensa-esp32-espidf --features esp32-embassy
```

❌ **Cannot Mix in Root Workspace**:
```bash
# This FAILS: riscv-rt conflict
cd /path/to/root
cargo build  # member resolution pulls both rp235x-hal & esp-hal
```

### Crate Dependencies

**task-watchdog-core**: Zero platform dependencies
- `portable-atomic` (no_std atomic support)
- `critical-section` (no_std synchronization)
- Optional: `defmt` for logging

**task-watchdog-rp**: RP-specific
- `task-watchdog-core` (path dep)
- `rp2040-hal` & `rp235x-hal` (mutually exclusive)
- `embassy-rp` (for async)

**task-watchdog-stm32/nrf/esp32**: Similar pattern
- Each depends on `task-watchdog-core`
- Each has its own HAL/Embassy deps
- NO cross-platform dependencies

### API Stability

All platform crates maintain identical APIs (to the original monolithic implementation):
- `WatchdogRunner<I>` / `WatchdogRunner<I, N>` - async wrapper
- `watchdog_run()` - task function
- Clock & HardwareWatchdog trait implementations

### Usage

Users depend directly on their platform crate and import from both:

```toml
# Cargo.toml
[dependencies]
task-watchdog-rp = { version = "0.1", features = ["rp2350-embassy"] }
```

```rust
// In your code
use task_watchdog_core::{WatchdogConfig, Id};  // Types and traits
use task_watchdog_rp::embassy_rp::WatchdogRunner;  // Platform implementation
```

### Examples & Build Scripts

**examples/Cargo.toml** references the appropriate platform crate:

```toml
[dependencies]
task-watchdog-rp = { path = "../crates/task-watchdog-rp", features = ["rp2040-embassy"] }
```

**scripts/build-lib.sh** builds each platform independently:

```bash
# Build RP2350
cd crates/task-watchdog-rp
cargo build --target thumbv8m.main-none-eabihf --features rp2350-embassy
```

### Publishing Strategy

Each crate publishes independently to crates.io with users selecting their platform:

```toml
# Pico 2 (RP2350)
task-watchdog-rp = { version = "0.1", features = ["rp2350-embassy"] }

# STM32 Bluepill
task-watchdog-stm32 = { version = "0.1", features = ["stm32-embassy"] }

# nRF52840
task-watchdog-nrf = { version = "0.1", features = ["nrf-embassy"] }

# ESP32
task-watchdog-esp32 = { version = "0.1", features = ["esp32-embassy"] }
```

## Verification

✅ **RP2350 builds in isolation** without ESP32 dependencies  
✅ **ESP32 builds in isolation** without RP2350 dependencies  
✅ **Root workspace** (core only) builds without platform conflicts  
✅ **All APIs preserved** from monolithic implementation  

## Summary

By splitting into platform-isolated crates with shared core logic, the library now:
- ✅ Supports all platforms simultaneously (via separate crate per platform)
- ✅ Avoids native library linking conflicts (no multi-version riscv-rt problem)
- ✅ Shares platform-agnostic business logic (core crate)
- ✅ Maintains backward-compatible APIs (same trait/struct interface)
- ✅ Simplifies dependency management (users import directly from platform crate)
- ✅ Eliminates unnecessary abstraction layers (no meta-crate indirection)
