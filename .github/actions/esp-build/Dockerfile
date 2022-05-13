# FROM alpine:3.10
FROM sergiogasquez/esp-rs-env:esp32

USER root

ENV RUSTUP_HOME=/home/esp/.rustup
ENV CARGO_HOME=/home/esp/.cargo

COPY LICENSE README.md /

COPY entrypoint.sh /entrypoint.sh

ENTRYPOINT ["/entrypoint.sh"]
