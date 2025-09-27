FROM artixlinux/artixlinux:base-dinit

# First install Arch Linux support
RUN mkdir -p /etc/pacman.d/gnupg && \
    pacman-key --init && \
    pacman-key --populate artix

RUN pacman -Syu \
    --noconfirm --needed \
    --disable-sandbox \
    artix-archlinux-support \
    archlinux-mirrorlist \
    archlinux-keyring

RUN cat >> /etc/pacman.conf <<EOB
[extra]
Include = /etc/pacman.d/mirrorlist-arch
EOB

RUN pacman -Syyu && pacman-key --populate archlinux

# Install other required packages
RUN pacman -S --noconfirm --needed \
    --disable-sandbox \
    devtools \
    base-devel \
    git \
    wget \
    python \
    openssh \
    rsync \
    ccache \
    sudo \
    && rm -rf /var/cache/pacman/pkg/*

# Add build user and give passwordless sudo access
RUN useradd -m -G wheel builder && \
    echo "builder ALL=(ALL) NOPASSWD: ALL" >> /etc/sudoers && \
    echo "Defaults env_keep += \"GNUPGHOME\"" >> /etc/sudoers

# Create build directories and set permissions
RUN mkdir -p /build /repo /build/chroot && \
    chown -R builder:builder /build /repo /build/chroot && \
    chmod 755 /build /repo /build/chroot


# Switch to build user for subsequent operations
USER builder
WORKDIR /build

# Set up build environment variables
ENV PATH="/usr/bin/vendor_perl:/usr/bin/site_perl:/usr/bin/core_perl:/usr/bin:$PATH" \
    GNUPGHOME="/home/builder/.gnupg" \
    PACKAGER="Artix Custom Repository <custom@artixlinux.org>" \
    MAKEFLAGS="-j$(nproc)"

# Create GPG directory for builder
RUN mkdir -p ${GNUPGHOME} && \
    chmod 700 ${GNUPGHOME}

# Copy entrypoint script
COPY --chown=builder:builder entrypoint.sh /usr/local/bin/entrypoint.sh
RUN chmod +x /usr/local/bin/entrypoint.sh

VOLUME ["/build", "/repo"]
ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
