FROM sergiogasquez/esp-rs-env:esp32

USER root

ENV RUSTUP_HOME=/home/esp/.rustup
ENV CARGO_HOME=/home/esp/.cargo

ENV PATH=/home/esp/.cargo/bin:$PATH

COPY entrypoint.sh /entrypoint.sh

ENTRYPOINT ["/entrypoint.sh"]
