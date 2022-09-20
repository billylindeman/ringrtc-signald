FROM ubuntu:latest AS build

RUN apt-get update && apt-get install -y git curl python3 build-essential make \ 
    && rm -rf /var/lib/apt/lists/* 

WORKDIR /opt/depot_tools
ENV PATH "/opt/depot_tools:${PATH}"
RUN git clone https://chromium.googlesource.com/chromium/tools/depot_tools.git .

# Install Rust
RUN curl https://sh.rustup.rs -sSf > /tmp/rustup-init.sh \
    && chmod +x /tmp/rustup-init.sh \
    && sh /tmp/rustup-init.sh -y \
    && rm -rf /tmp/rustup-init.sh
ENV PATH="/root/.cargo/bin:${PATH}"

WORKDIR /usr/src/ringrtc

COPY bin bin 
COPY config config
RUN ./bin/prepare-workspace unix

COPY . .
ENV PLATFORM unix
RUN make cli
