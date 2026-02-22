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

# AI tools
RUN npm install -g @anthropic-ai/claude-code @openai/codex

# Non-root user with configurable UID
ARG USER_UID=1000
RUN useradd -m -u ${USER_UID} -s /bin/bash hltm
USER hltm

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
