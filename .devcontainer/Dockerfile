FROM ubuntu:22.04 AS base

# Create non-root user
ENV USER_ID=1000
ENV GROUP_ID=1000
RUN addgroup --gid $GROUP_ID rctrl
RUN adduser --disabled-password --gecos '' --uid $USER_ID --gid $GROUP_ID rctrl

# Fix locale issues
RUN apt-get update
RUN DEBIAN_FRONTEND=noninteractive apt-get install -qy locales
RUN locale-gen en_US.UTF-8

FROM base as dev_depends

# Install dev dependencies
RUN apt-get update
RUN DEBIAN_FRONTEND=noninteractive apt-get install -qy jq \
                                                       git \
                                                       wget \
                                                       curl \
                                                       pkg-config \
                                                       libssl-dev \
                                                       build-essential \
                                                       libfontconfig1-dev \
                                                       gcc-aarch64-linux-gnu \
                                                       software-properties-common

# Install rust
ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH
RUN curl https://sh.rustup.rs -sSf | bash -s -- -y
RUN chmod -R a+w $RUSTUP_HOME $CARGO_HOME;
RUN rustup target add aarch64-unknown-linux-gnu

# Install rust/WASM dependencies
USER rctrl
RUN rustup component add rls rust-analysis rust-src
RUN rustup target add wasm32-unknown-unknown
RUN cargo install wasm-bindgen-cli

# Install trunk
RUN cargo install --locked trunk

# influxdb
# REFERENCE: https://docs.influxdata.com/influxdb/v2.0/install/?t=Linux
USER root
ENV INFLUX_RELEASE=2.2.0
ENV INFLUX_CL_RELEASE=2.3.0
RUN arch="$(uname -m)"; \
    case "$arch" in \
        aarch64) export INFLUX_ARCH='arm64' ;; \
        x86_64) export INFLUX_ARCH='amd64' ;; \
    esac \
    && wget https://dl.influxdata.com/influxdb/releases/influxdb2-${INFLUX_RELEASE}-${INFLUX_ARCH}.deb \
    && dpkg -i influxdb2-${INFLUX_RELEASE}-${INFLUX_ARCH}.deb \
    && rm influxdb2-${INFLUX_RELEASE}-${INFLUX_ARCH}.deb \
    && wget https://dl.influxdata.com/influxdb/releases/influxdb2-client-${INFLUX_CL_RELEASE}-linux-${INFLUX_ARCH}.tar.gz \
    && tar xvzf influxdb2-client-${INFLUX_CL_RELEASE}-linux-${INFLUX_ARCH}.tar.gz \
    && cp influxdb2-client-${INFLUX_CL_RELEASE}-linux-${INFLUX_ARCH}/influx /usr/local/bin/ \
    && rm influxdb2-client-${INFLUX_CL_RELEASE}-linux-${INFLUX_ARCH}.tar.gz \
    && rm -r influxdb2-client-${INFLUX_CL_RELEASE}-linux-${INFLUX_ARCH}

# Add influx credentials file
USER rctrl
RUN mkdir /home/rctrl/influx
COPY resources/influx/credentials.toml /home/rctrl/influx/credentials.toml

# Add influx configuration file
COPY resources/influx/config.toml /home/rctrl/influx/config.toml
ENV INFLUXD_CONFIG_PATH = "/home/rctrl/influx"

# Bring in entrypoint script from local machine
# Make sure script is executable with chmod -x
COPY resources/setup.sh /home/rctrl/setup.sh
USER root
RUN chmod +x /home/rctrl/setup.sh