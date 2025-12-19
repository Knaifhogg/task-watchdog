# task-watchdog AI Coding Instructions

## Architecture Overview

**task-watchdog** multiplexes multiple independent task watchdogs into a single hardware watchdog timer, preventing system lockups when embedded tasks fail to respond.

### Core Design Pattern

The library uses a trait-based abstraction system:
- `HardwareWatchdog<C>` - Manages actual hardware watchdog peripheral (start, feed, reset)
- `Clock` - Provides time tracking with platform-specific implementations
- `Id` trait - Task identifiers (must implement `Clone, Copy, PartialEq, Eq, Debug`)
- `WatchdogConfig<C>` - Configuration with `hardware_timeout` and `check_interval`

### Two-Tier API Strategy

1. **Generic Core** (`crates/task-watchdog-core/`): Platform-agnostic implementation for custom use cases
   - `Watchdog<I, W, C>` (alloc feature with BTreeMap)
   - `Watchdog<I, N, W, C>` (no_alloc with fixed-size array of N tasks)

2. **Platform-Specific Implementations**: Separate crates for each platform
   - `crates/task-watchdog-rp/`: RP2040/RP2350 (Embassy async + HAL sync)
   - `crates/task-watchdog-stm32/`: STM32 (Embassy async)
   - `crates/task-watchdog-nrf/`: nRF52840 (Embassy async)
   - `crates/task-watchdog-esp32/`: ESP32 (Embassy async)
   - Each implements `WatchdogRunner<I>` wrapper with mutex for thread safety

### Workspace Structure & Dependency Isolation

**Critical Architecture Decision**: Platform crates are **NOT** workspace members in root `Cargo.toml` to avoid native library linking conflicts.

**Root Workspace** (resolves core only):
```
[workspace]
members = ["crates/task-watchdog-core"]
exclude = ["crates/task-watchdog-{rp,stm32,nrf,esp32}"]
```

**Platform Crates** (standalone, built in isolation):
- Located in `crates/task-watchdog-{rp,stm32,nrf,esp32}/` 
- Each is a **standalone Cargo project** with its own `Cargo.toml` and dependency resolution
- **NOT** included in root workspace members to prevent `riscv-rt` conflict (RP needs v0.12, ESP32 needs v0.16)

**Build Strategy**:
```bash
# ✅ Correct: Platform-specific build (no conflicts)
cd crates/task-watchdog-rp
cargo build --target thumbv8m.main-none-eabihf --features rp2350-embassy

# ❌ Avoid: Root workspace with multiple platforms (native lib conflict)
cargo build  # From root with multiple platform members = riscv-rt mismatch error
```

### Feature Matrix (Per-Platform)

Each platform crate has its own feature flags. Users select ONE platform crate:

**RP Crate** (`task-watchdog-rp`):
- `rp2040-embassy` - RP2040/Pico async
- `rp2350-embassy` - RP2350/Pico 2 async
- `rp2040-hal` - RP2040/Pico sync (no Embassy)
- `rp2350-hal` - RP2350/Pico 2 sync (no Embassy)
- `defmt-embassy-rp` - Combine with async features for logging

**STM32 Crate** (`task-watchdog-stm32`):
- `stm32-embassy` - STM32 async
- `defmt-embassy-stm32` - Enable logging

**nRF Crate** (`task-watchdog-nrf`):
- `nrf-embassy` - nRF52840 async
- `defmt-embassy-nrf` - Enable logging

**ESP32 Crate** (`task-watchdog-esp32`):
- `esp32-embassy` - ESP32 async
- `defmt-embassy-esp32` - Enable logging

**Shared Across All Platforms**:
- `alloc` - Enable BTreeMap-based dynamic task management (not recommended for embedded)
- `defmt` - Enable defmt logging (maps to platform-specific defmt feature)

## Developer Workflows

### Building

```bash
# Build RP2350 async (from platform crate directory, isolated)
cd crates/task-watchdog-rp
cargo build --target thumbv8m.main-none-eabihf --features rp2350-embassy

# Build ESP32 async (from platform crate directory, isolated)
cd crates/task-watchdog-esp32
cargo build --target xtensa-esp32-espidf --features esp32-embassy

# Build root workspace (core + meta only, no platform conflicts)
cd /path/to/root
cargo build -p task-watchdog-core

# ALL scripts: Automatically cd into platform crate directories
scripts/build-all.sh  # Builds each platform from its own crate
```

**IMPORTANT**: Do NOT build workspace-wide from root when multiple platforms are members—this triggers riscv-rt conflicts. Always use `cd crates/task-watchdog-<platform>` for platform builds.

### Flashing Examples

```bash
scripts/flash-async-pico.sh        # RP2040 with debug probe
scripts/flash-async-pico2.sh       # RP2350
scripts/flash-async-stm32f103c8.sh # STM32 bluepill
scripts/flash-async-nrf52840.sh    # nRF52840

# For sync (RP2040/RP2350 only)
scripts/flash-sync-pico.sh
scripts/flash-sync-pico2.sh
```

### ESP32 Build (needs extra setup)

```bash
# Install ESP tooling
cargo install espup espflash cargo-espflash

# Build from platform crate directory
cd crates/task-watchdog-esp32
. ~/export-esp.sh
~/.rustup/toolchains/esp/bin/cargo build --features esp32-embassy --target xtensa-esp32-espidf
```

### Key Targets

- RP2040/Pico: `thumbv6m-none-eabi`
- RP2350/Pico 2: `thumbv8m.main-none-eabihf`
- STM32: `thumbv7m-none-eabi`
- nRF52840: `thumbv7em-none-eabi`
- ESP32: `xtensa-esp32-espidf` or `xtensa-esp32-none-elf`

## Critical Patterns & Conventions

### Task ID Pattern

Define an enum with required trait derives:

```rust
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum TaskId {
    MainTask,
    SensorTask,
}
impl Id for TaskId {} // Empty impl - derives handle requirements
```
**Why**: Task IDs are compared by discriminant (not value) for equality checks. Must be `Copy` for static sharing.

### Two Allocation Modes

**no_alloc (recommended for embedded)**:
```rust
let watchdog = Watchdog::<TaskId, NUM_TASKS, HwWatchdog, Clock>::new(hw, config, clock);
// Fixed-size array: compile-time task limit enforced
```

**alloc mode**:
```rust
let watchdog = Watchdog::<TaskId, HwWatchdog, Clock>::new(hw, config, clock); // BTreeMap
// Requires `alloc` feature and Ord trait on TaskId
```

Register first, then fail silently on no_alloc vs panicking on alloc.

### Async API (Embassy)

Static cell pattern is required for task sharing:

```rust
static WATCHDOG: StaticCell<WatchdogRunner<TaskId, NUM_TASKS>> = StaticCell::new();

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let watchdog = WatchdogRunner::new(hw_watchdog_peripheral, config);
    let watchdog = WATCHDOG.init(watchdog);
    
    watchdog.register_task(&TaskId::Main, Duration::from_millis(2000)).await;
    spawner.must_spawn(watchdog_task(watchdog)); // Spawn watchdog thread
    spawner.must_spawn(my_task(watchdog));       // Spawn app task
}

// Watchdog monitoring task (never terminates)
#[embassy_executor::task]
async fn watchdog_task(w: &'static WatchdogRunner<TaskId, NUM_TASKS>) -> ! {
    watchdog_run(w.create_task()).await  // Handles checking and feeding
}

// Application task (must feed regularly)
#[embassy_executor::task]
async fn my_task(w: &'static WatchdogRunner<TaskId, NUM_TASKS>) -> ! {
    loop {
        w.feed(&TaskId::Main).await;
        Timer::after(Duration::from_millis(1000)).await;
    }
}
```

### Sync API (RP HAL only)

Manual polling loop in main, no Embassy:

```rust
let mut watchdog = Watchdog::<TaskId, NUM_TASKS, RpHalWatchdog, RpHalClock>::new(hw, config, clock);
watchdog.register_task(&TaskId::Main, duration);
watchdog.start(); // Starts hardware watchdog

// In your main loop
watchdog.feed(&TaskId::Main);
if watchdog.check() {
    watchdog.trigger_reset(); // Manual system reset if task starved
}
```

### Configuration Timeout Relationship

**Must hold**: `check_interval < hardware_timeout` (typically 1/5 ratio)

- `hardware_timeout`: Duration before hardware resets system if not fed (e.g., 5s)
- `check_interval`: How often watchdog task checks if any task has starved (e.g., 1s)

If check_interval ≥ hardware_timeout, the watchdog may reset before detecting starvation.

## Integration Points & External Dependencies

### Peripheral Bindings (feature-gated)

- **RP2040/RP2350**: `rp2040-hal`, `rp235x-hal`, `embassy-rp`
- **STM32**: `embassy-stm32` (IWDG - Independent Watchdog)
- **nRF**: `embassy-nrf` (WDT - Watchdog Timer)
- **ESP32**: `esp-hal` (Timer Group watchdog)

Each platform's watchdog has different timeout ranges and granularity—handled in platform-specific `HardwareWatchdog` implementations.

### Critical Traits to Implement

When adding new platforms, implement:

1. **`HardwareWatchdog<C: Clock>`**:
   - `start(C::Duration)` - Enable hardware watchdog
   - `feed()` - Prevent timeout
   - `trigger_reset() -> !` - Force reset
   - `reset_reason() -> Option<ResetReason>` - Optional: detect watchdog resets

2. **`Clock`**:
   - `now() -> Instant` - Current time
   - `elapsed_since(Instant) -> Duration`
   - `has_elapsed(Instant, Duration) -> bool`
   - `duration_from_millis(u64) -> Duration`

### Logging (Optional)

- With `defmt` feature: Uses defmt macros (info!, warn!, error!, debug!)
- Without it: Macros compile to no-ops (see `log_impl` module)
- Use for task registration/deregistration, starvation detection

## Cross-File Communication

- **examples/src/bin/intro.rs**: Minimal 2-task async example (RP2040/2350/STM32/nRF/ESP32)
- **examples/src/bin/embassy.rs**: Full embassy example with more features
- **examples/src/bin/rp-sync.rs**: RP2040/2350 synchronous (no Embassy)
- **scripts/build-*.sh**: Drive builds with proper feature combinations
- **CHANGELOG.md**: Track breaking changes between versions (0.1.x currently)

## Common Implementation Rules

1. **Task Deregistration**: Use `core::mem::discriminant()` comparison—ignore enum payload values
2. **Panic Points**: Allocation failures (`no_alloc` safe), timeout misconfiguration (task would starve forever)
3. **Mutex Strategy**: All async wrappers use `embassy_sync::Mutex<CriticalSectionRawMutex, RefCell<>>` for static safety
4. **Static Lifetimes**: Tasks must be `'static` for ESP32 (potential system limitation)
5. **Defmt Logging Context**: When modifying task starvation detection, ensure error messages include TaskId for debugging

## Testing & Validation

- Build all feature combinations: `scripts/build-all.sh`
- Flash and verify on hardware: `scripts/flash-*.sh` (requires debug probe)
- Examples verify correct multiplexing on at least one task per platform
- No standard `cargo test` (embedded bare-metal environment)
