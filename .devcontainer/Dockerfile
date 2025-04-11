# Base image
ARG VARIANT=bookworm-slim
FROM debian:${VARIANT}
ENV DEBIAN_FRONTEND=noninteractive
ENV LC_ALL=C.UTF-8
ENV LANG=C.UTF-8

# Install dependencies
RUN apt-get update \
    && apt-get install -y git curl llvm-dev libclang-dev clang unzip \
    libusb-1.0-0 libssl-dev libudev-dev pkg-config xz-utils \
    && apt-get clean -y && rm -rf /var/lib/apt/lists/* /tmp/library-scripts


# Setup shell for user
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
RUN usermod --shell /bin/zsh root

COPY shell/.zshrc shell/.welcome.sh /root/
# Coder has some issue with adding DOS line endings
RUN sed -i "s/HOMEPATH/root/" /root/.zshrc && \
    git clone --single-branch --depth 1 https://github.com/robbyrussell/oh-my-zsh.git /root/.oh-my-zsh

ARG POWERLEVEL10K_VERSION=v1.16.1
COPY shell/.p10k.zsh /root/
RUN git clone --branch ${POWERLEVEL10K_VERSION} --single-branch --depth 1 https://github.com/romkatv/powerlevel10k.git /root/.oh-my-zsh/custom/themes/powerlevel10k && \
    rm -rf /root/.oh-my-zsh/custom/themes/powerlevel10k/.git

# Install rustup
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- \
    --default-toolchain none -y --profile minimal

# Update envs
ENV PATH=${PATH}:/root/.cargo/bin

# Install probe rs
RUN curl --proto '=https' --tlsv1.2 -LsSf https://github.com/probe-rs/probe-rs/releases/latest/download/probe-rs-tools-installer.sh | sh

WORKDIR /rustpill