#!/usr/bin/env bash

set -e
set -o pipefail

export PATH=$PATH:/home/esp/.cargo/bin
. /home/esp/export-rust.sh
export IDF_TOOLS_PATH=/home/esp/.espressif
. /home/esp/.espressif/frameworks/esp-idf/export.sh

# reinstall cargo-espflash
# re: https://github.com/SergioGasquez/esp-rs-container/issues/4#issuecomment-1126363859
cargo install cargo-espflash --force

bash -c "set -e;  set -o pipefail; $1"

ls -al
