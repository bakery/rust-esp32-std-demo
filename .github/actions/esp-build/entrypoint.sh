#!/bin/sh -l

echo "hello $1"
id -u -n

export PATH=$PATH:/home/esp/.cargo/bin

echo "$PATH"

ls -al /home/esp/.cargo/bin

# cargo --help

#ls /home/esp -al
#chown -R root:root /home/esp
#ls /home/esp -al
# /home/esp/export-rust.sh
# env
# /home/esp/.cargo/bin/rustup toolchain list
# rustup toolchain list

# cargo install cargo-espflash

export RUST_ESP32_STD_DEMO_WIFI_SSID=ssid; export RUST_ESP32_STD_DEMO_WIFI_PASS=pass; cargo +esp build

# export RUST_ESP32_STD_DEMO_WIFI_SSID=ssid; export RUST_ESP32_STD_DEMO_WIFI_PASS=pass; cargo +esp espflash --partition-table ./partitions.csv save-image firmware-22.bin
