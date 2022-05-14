#!/usr/bin/env bash

set -e
set -o pipefail

export PATH=$PATH:/home/esp/.cargo/bin
. /home/esp/export-rust.sh
export IDF_TOOLS_PATH=/home/esp/.espressif
. /home/esp/.espressif/frameworks/esp-idf/export.sh

chmod +x /home/esp/.cargo/bin/*
ls -al /home/esp/.cargo/bin
/home/esp/.cargo/bin/cargo-espflash --help

bash -c "set -e;  set -o pipefail; $1"

# export RUST_ESP32_STD_DEMO_WIFI_SSID=ssid; export RUST_ESP32_STD_DEMO_WIFI_PASS=pass; cargo +esp build
# export RUST_ESP32_STD_DEMO_WIFI_SSID=ssid; export RUST_ESP32_STD_DEMO_WIFI_PASS=pass; cargo +esp espflash --partition-table ./partitions.csv save-image firmware-22.bin
