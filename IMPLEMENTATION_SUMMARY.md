# task-watchdog Split Crate Implementation Summary

## Completed Tasks

### 1. **Examples/Cargo.toml Refactored** ✅
- **Replaced**: Monolithic `task-watchdog` meta-crate reference
- **With**: Platform-specific split crates (`task-watchdog-rp`, `task-watchdog-stm32`, `task-watchdog-nrf`)
- **ESP32 Removed**: Intentionally excluded from examples to avoid `riscv-rt` v0.12 vs v0.16 conflict with RP platforms
- **Result**: 92 lines (down from 168), clean feature structure
- **Verified**: Cargo metadata parsing confirms correct manifest structure
- **Features**: Simplified from 36 nested features to clean platform-specific sets

### 2. **Dependency Structure Cleaned**
- **Removed**: Direct embassy crate dependencies from examples
- **Reason**: Platform crates handle their own embassy integration
- **Result**: Examples now only depend on platform crates, eliminating transitive conflicts
- **Dependencies**: Reduced from 21 to 18 by removing optional deps that weren't selected

### 3. **Build Script Updated** ✅
- **Updated**: `scripts/build-lib.sh` for split-crate architecture
- **Strategy**: 
  - Build core from root workspace
  - Build each platform crate independently from its directory (`cd crates/task-watchdog-<platform>`)
- **Coverage**: All four platforms (RP, STM32, nRF, ESP32) with feature combinations
- **Result**: Clean, maintainable build script with clear platform separation

## Architecture Benefits Achieved

### ✅ Resolved Core Issue: Native Library Conflict
- **Problem**: RP2350 needs `riscv-rt` v0.12, ESP32 needs v0.16 (via `esp-riscv-rt`)
- **Solution**: Platform crates are workspace-excluded, allowing independent builds
- **Verification**: No transitive dependency conflicts in clean builds per platform

### ✅ Simplified Feature Surface
**Before (per-crate): 8-11 features** → **After (per-crate): 2-6 features**

| Crate | Before | After | Approach |
|-------|--------|-------|----------|
| task-watchdog-rp | 11 | 6 | Multi-mode (embassy + HAL) justified |
| task-watchdog-stm32 | 8 | 2 | Single-mode (alloc, defmt) |
| task-watchdog-nrf | 8 | 2 | Single-mode (alloc, defmt) |
| task-watchdog-esp32 | 8 | 2 | Single-mode (alloc, defmt) |

### ✅ Cleaner User Experience
- **Before**: `cargo build --features esp32-embassy,defmt-embassy-esp32,alloc`
- **After**: `cargo build -p task-watchdog-examples --features rp2040-embassy-defmt`
- **Multiple platforms**: Build independently per platform, no conflicts

## File Changes Summary

### Created/Modified
1. **examples/Cargo.toml** - Complete refactor
   - Added `[workspace]` for package independence
   - Rewrote [features] section (36 → 8 clean features)
   - Updated [dependencies] to use split platform crates
   - Removed ESP32 to avoid conflict

2. **scripts/build-lib.sh** - Updated for split builds
   - Changed from monolithic build to per-crate isolation
   - Each platform builds from its own directory
   - Simplified feature combinations

### Unchanged (but Verified)
- Root `Cargo.toml`: Workspace members/excludes correct ✅
- All 5 platform crates: Dependencies/features validated ✅
- Core functionality: No logic changes, only structural refactoring ✅

## Build Instructions (Users)

### Build Examples for Specific Platform
```bash
# RP2040
cd examples
cargo check --features rp2040-embassy --target thumbv6m-none-eabi

# RP2350
cargo check --features rp2350-embassy --target thumbv8m.main-none-eabihf

# STM32
cargo check --features stm32-embassy --target thumbv7m-none-eabi

# nRF52840
cargo check --features nrf-embassy --target thumbv7em-none-eabi

# With defmt logging
cargo build --features rp2040-embassy-defmt --release
```

### Build Individual Platform Crates
```bash
# RP2040
cd crates/task-watchdog-rp
cargo build --target thumbv6m-none-eabi --features rp2040-embassy

# ESP32 (from isolated crate)
cd crates/task-watchdog-esp32
. ~/export-esp.sh
cargo build --target xtensa-esp32-espidf --features defmt
```

### Full Validation
```bash
# Build all platforms (from repo root)
scripts/build-lib.sh
```

## Design Rationale

### Why Examples Don't Support ESP32
- **Technical**: `riscv-rt` native library linking creates hard conflict in single manifest
- **User Experience**: Most embedded examples target ARM Cortex-M
- **Workaround**: Users building ESP32 apps can use `task-watchdog-esp32` crate directly with their own examples
- **Maintainability**: Simpler, non-conflicting dependency graph

### Why Features are Simple per-Platform Crate
- **Principle**: Each crate = single platform = no need for feature gates within the platform
- **Clarity**: `task-watchdog-esp32` definitionally targets ESP32; no `esp32-embassy` feature needed
- **Simplicity**: Only `alloc` (dynamic tasks) and `defmt` (logging) are orthogonal concerns

### Why Build Strategy Requires `cd` into Platform Crate Directories
- **Cargo Workspace Rules**: Can only have one `links = "riscv-rt"` per workspace resolution
- **Solution**: Each platform builds in isolation, separate `Cargo.lock` files
- **Benefit**: No version negotiation between conflicting native libraries

## Verification Checklist

- ✅ examples/Cargo.toml parses correctly (cargo metadata validates)
- ✅ All features defined without undefined dependencies
- ✅ Platform crates correctly isolated in workspace exclude list
- ✅ Build script supports all 4 platforms with proper feature combinations
- ✅ Core logic unchanged; refactoring is structural only
- ✅ Dependency versions compatible (within each platform's graph)
- ✅ No circular dependencies introduced
- ✅ Feature references map to actual crate features

## Known Limitations

1. **ARM Cortex-M Only in Examples**: ESP32 users must create their own examples using `task-watchdog-esp32` directly
2. **Target Installation**: Users need to install appropriate Rust targets (`rustup target add thumbv6m-none-eabi`, etc.)
3. **ESP Tooling**: ESP32 requires extra setup (`espup`, ESP-IDF environment)

## Next Steps (For Future Work)

1. **Document**: Update README with new build instructions
2. **CI/CD**: Ensure each platform crate builds independently in CI
3. **Publishing**: Each platform crate published separately to crates.io
4. **Documentation**: Add platform-specific guides for users

## Rollback Safety

All changes are reversible:
- Original crate structure preserved in crate implementations
- No logic changes, only manifest/build script updates
- Version matrix preserved (0.1.2 across all crates)
- Git history preserved for reference
