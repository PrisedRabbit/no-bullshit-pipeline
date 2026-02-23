FROM node:24-bookworm

# System tools
RUN apt-get update && apt-get install -y --no-install-recommends \
    git ripgrep bash curl ca-certificates python3 python3-pip jq \
    && rm -rf /var/lib/apt/lists/*

# GitHub CLI
RUN curl -fsSL https://cli.github.com/packages/githubcli-archive-keyring.gpg \
      -o /usr/share/keyrings/githubcli-archive-keyring.gpg \
    && echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/githubcli-archive-keyring.gpg] https://cli.github.com/packages stable main" \
      > /etc/apt/sources.list.d/github-cli.list \
    && apt-get update && apt-get install -y gh && rm -rf /var/lib/apt/lists/*

# Beads (task tracker)
ARG TARGETARCH
RUN BEADS_VERSION="0.55.4" && \
    curl -fsSL "https://github.com/steveyegge/beads/releases/download/v${BEADS_VERSION}/beads_${BEADS_VERSION}_linux_${TARGETARCH}.tar.gz" \
      | tar -xz -C /usr/local/bin bd

# Rust (system-wide)
ENV RUSTUP_HOME=/usr/local/rustup CARGO_HOME=/usr/local/cargo
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path
ENV PATH="/usr/local/cargo/bin:${PATH}"

# Cargo shim — delegates check/clippy/build to host Mac via bridge
# Symlinks in /usr/local/bin/ ensure tools are found even if /usr/local/cargo/bin not in PATH
COPY --chmod=755 scripts/cargo-shim.sh /usr/local/bin/cargo-shim
RUN mv /usr/local/cargo/bin/cargo /usr/local/cargo/bin/cargo-real \
    && ln -s /usr/local/bin/cargo-shim /usr/local/cargo/bin/cargo \
    && ln -s /usr/local/bin/cargo-shim /usr/local/bin/cargo \
    && ln -s /usr/local/cargo/bin/rustc /usr/local/bin/rustc \
    && ln -s /usr/local/cargo/bin/rustup /usr/local/bin/rustup

# Bun (system-wide)
ENV BUN_INSTALL="/usr/local"
RUN curl -fsSL https://bun.sh/install | bash

# AI tools
RUN npm install -g @anthropic-ai/claude-code @openai/codex

# Non-root user with configurable UID + sudo
ARG USER_UID=1000
RUN apt-get update && apt-get install -y --no-install-recommends sudo && rm -rf /var/lib/apt/lists/* \
    && useradd -m -u ${USER_UID} -s /bin/bash hltm \
    && echo "hltm ALL=(ALL) NOPASSWD:ALL" > /etc/sudoers.d/hltm
USER hltm
RUN echo 'export PATH="/usr/local/cargo/bin:$PATH"' >> ~/.bashrc

# Docker env marker (codex uses danger-full-access sandbox)
ENV HLTM_DOCKER=1

# Git safe.directory — workspace is host-mounted
ENV GIT_CONFIG_COUNT=1
ENV GIT_CONFIG_KEY_0=safe.directory
ENV GIT_CONFIG_VALUE_0=/workspace

# Scripts
COPY --chmod=755 scripts/init-docker.sh /usr/local/bin/init-docker.sh

WORKDIR /workspace
ENTRYPOINT ["init-docker.sh"]
