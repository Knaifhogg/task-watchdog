#!/bin/bash
set -e
set -x

# Include common build script stuff
source scripts/build-common.sh

# Build embassy example ...
EXAMPLE=embassy

cargo run --manifest-path=$EXAMPLES_MANIFEST_PATH --bin $EXAMPLE --target $NRF_TARGET --features nrf-embassy-defmt