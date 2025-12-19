#!/bin/bash
set -e
set -x

# Include common build script stuff
source scripts/build-common.sh

# Build example ...
EXAMPLE=rp-sync

# ... for Pico 2
TARGET=$RP2350_TARGET

cargo run --manifest-path=$EXAMPLES_MANIFEST_PATH --bin $EXAMPLE --target $TARGET --features rp2350-hal-defmt