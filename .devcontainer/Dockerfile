# Base image
ARG VARIANT=bookworm-slim
FROM debian:${VARIANT}
ENV DEBIAN_FRONTEND=noninteractive
ENV LC_ALL=C.UTF-8
ENV LANG=C.UTF-8

# Arguments
ARG CONTAINER_USER=qlab
ARG USER_UID=1000
ARG USER_GID=1000

# Install dependencies
RUN apt-get update \
    && apt-get install -y git curl llvm-dev libclang-dev clang unzip \
    libusb-1.0-0 libssl-dev libudev-dev pkg-config \
    && apt-get clean -y && rm -rf /var/lib/apt/lists/* /tmp/library-scripts

# Set users
RUN apt-get update -y && \
    apt-get install -y --no-install-recommends sudo && \
    apt-get clean -y && rm -rf /var/lib/apt/lists/* /tmp/library-scripts
RUN addgroup --gid $USER_GID user && \
    useradd $CONTAINER_USER --shell /bin/sh --create-home --uid $USER_UID --gid $USER_GID && \
    chown -R $USER_UID:$USER_GID /home/$CONTAINER_USER && \
    mkdir -p /etc/sudoers.d && \
    echo $CONTAINER_USER ALL=\(root\) NOPASSWD:ALL > /etc/sudoers.d/$CONTAINER_USER && \
    chmod 0440 /etc/sudoers.d/$CONTAINER_USER && \
    rm /var/log/faillog /var/log/lastlog
USER ${CONTAINER_USER}
WORKDIR /home/${CONTAINER_USER}

# Setup shell for root and user
ENTRYPOINT [ "/bin/zsh" ]
RUN apt-get update -y && \
    apt-get install -y --no-install-recommends zsh nano locales && \
    apt-get autoremove -y && \
    apt-get clean -y && \
    rm -r /var/cache/* /var/lib/apt/lists/*
ENV EDITOR=nano \
    LANG=en_US.UTF-8 \
    # MacOS compatibility
    TERM=xterm
RUN echo "LC_ALL=en_US.UTF-8" >> /etc/environment && \
    echo "en_US.UTF-8 UTF-8" >> /etc/locale.gen && \
    echo "LANG=en_US.UTF-8" > /etc/locale.conf && \
    locale-gen en_US.UTF-8
RUN usermod --shell /bin/zsh $CONTAINER_USER && \
    usermod --shell /bin/zsh root

# Install rustup
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- \
    --default-toolchain none -y --profile minimal

# Install probe rs
RUN curl --proto '=https' --tlsv1.2 -LsSf https://github.com/probe-rs/probe-rs/releases/latest/download/probe-rs-tools-installer.sh | sh

# prepare ST-Link probe
ADD ./69-probe-rs.rules /etc/udev/rules.d/69-probe-rs.rules
RUN sudo udevadm control --reload-rules && sudo udevadm trigger

# Update envs
ENV PATH=${PATH}:/home/${CONTAINER_USER}/.cargo/bin

